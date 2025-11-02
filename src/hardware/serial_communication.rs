use serialport::{available_ports, SerialPort, SerialPortType};
use std::time::Duration;
use std::io::{Read, Write};
use std::sync::mpsc;
use std::thread;
use crate::utils::angle::AngleDMS;

#[derive(Debug, Clone)]
pub enum ControllerCommand {
    Ping,
    GetId,
    Home,
    SetZero,
    /// Перемещение по оси X с указанием скорости
    /// Формат команды: MOVE X=<mm> F=<mm/s>\n
    Move { x_mm: f32, speed_mm_s: f32 },
    /// Установка угла через энкодер
    /// Формат команды: SETANG <deg|d:m:s>\n
    SetAngle { angle: AngleDMS },
}

#[derive(Debug, Clone)]
pub enum ControllerResponse {
    Pong,
    Id(String),
    HomeComplete,
    SetZeroComplete,
    MoveComplete,
    SetAngleComplete,
    Error(String),
}

#[derive(Debug)]
pub struct SerialController {
    port_name: String,
    baud_rate: u32,
    sender: Option<mpsc::Sender<ControllerCommand>>,
    receiver: Option<mpsc::Receiver<ControllerResponse>>,
}

impl SerialController {
    pub fn new(baud_rate: u32) -> Self {
        Self {
            port_name: String::new(),
            baud_rate,
            sender: None,
            receiver: None,
        }
    }

    /// Auto-detect available COM ports, filtering out virtual ports
    pub fn detect_ports() -> Vec<String> {
        match available_ports() {
            Ok(ports) => {
                ports
                    .into_iter()
                    .filter(|port| {
                        // Filter out virtual ports based on port type
                        match &port.port_type {
                            SerialPortType::UsbPort(_) => true,
                            SerialPortType::PciPort => true,
                            SerialPortType::BluetoothPort => false,
                            SerialPortType::Unknown => {
                                // For unknown types, check if it's likely a virtual port
                                !port.port_name.contains("Virtual")
                                    && !port.port_name.contains("COM_MAP")
                                    && !port.port_name.contains("VCP")
                            }
                        }
                    })
                    .map(|port| port.port_name)
                    .collect()
            }
            Err(_) => Vec::new(),
        }
    }

    /// Connect to a specific port
    pub fn connect(&mut self, port_name: &str) -> Result<(), String> {
        // Create channels for command/response communication
        let (cmd_tx, cmd_rx) = mpsc::channel::<ControllerCommand>();
        let (resp_tx, resp_rx) = mpsc::channel::<ControllerResponse>();

        // Store the sender and receiver
        self.sender = Some(cmd_tx);
        self.receiver = Some(resp_rx);
        self.port_name = port_name.to_string();

        // Spawn a thread to handle serial communication
        let port_name = port_name.to_string();
        let baud_rate = self.baud_rate;
        
        thread::spawn(move || {
            // Attempt to open the serial port
            let mut port = match serialport::new(&port_name, baud_rate)
                .timeout(Duration::from_millis(1000))
                .open() {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("Failed to open serial port {}: {}", port_name, e);
                        return;
                    }
                };

            // Handle incoming commands
            while let Ok(command) = cmd_rx.recv() {
                let response = match command {
                    ControllerCommand::Ping => {
                        // Согласно ТЗ: команда PING\n → ожидание ответа PONG\n
                        if let Err(e) = Self::send_command(&mut port, "PING") {
                            ControllerResponse::Error(format!("PING failed: {}", e))
                        } else {
                            // Read response for ping
                            match Self::read_response(&mut port) {
                                Ok(response) if response.trim() == "PONG" => ControllerResponse::Pong,
                                Ok(response) => ControllerResponse::Error(format!("Unexpected PING response: {}", response)),
                                Err(e) => ControllerResponse::Error(format!("PING response error: {}", e)),
                            }
                        }
                    }
                    ControllerCommand::GetId => {
                        if let Err(e) = Self::send_command(&mut port, "ID?") {
                            ControllerResponse::Error(format!("ID? failed: {}", e))
                        } else {
                            match Self::read_response(&mut port) {
                                Ok(response) => ControllerResponse::Id(response.trim().to_string()),
                                Err(e) => ControllerResponse::Error(format!("ID? response error: {}", e)),
                            }
                        }
                    }
                    ControllerCommand::Home => {
                        if let Err(e) = Self::send_command(&mut port, "HOME") {
                            ControllerResponse::Error(format!("HOME failed: {}", e))
                        } else {
                            // HOME might take some time, so we might want to wait for a specific response
                            match Self::read_response(&mut port) {
                                Ok(response) if response.trim() == "HOMING_COMPLETE" => ControllerResponse::HomeComplete,
                                Ok(response) => ControllerResponse::Error(format!("Unexpected HOME response: {}", response)),
                                Err(e) => ControllerResponse::Error(format!("HOME response error: {}", e)),
                            }
                        }
                    }
                    ControllerCommand::SetZero => {
                        if let Err(e) = Self::send_command(&mut port, "SETZERO") {
                            ControllerResponse::Error(format!("SETZERO failed: {}", e))
                        } else {
                            match Self::read_response(&mut port) {
                                Ok(response) if response.trim() == "ZERO_SET" => ControllerResponse::SetZeroComplete,
                                Ok(response) => ControllerResponse::Error(format!("Unexpected SETZERO response: {}", response)),
                                Err(e) => ControllerResponse::Error(format!("SETZERO response error: {}", e)),
                            }
                        }
                    }
                    ControllerCommand::Move { x_mm, speed_mm_s } => {
                        // Формат согласно ТЗ: MOVE X=<mm> F=<mm/s>\n
                        let command = format!("MOVE X={:.3} F={:.3}", x_mm, speed_mm_s);
                        if let Err(e) = Self::send_command(&mut port, &command) {
                            ControllerResponse::Error(format!("MOVE failed: {}", e))
                        } else {
                            match Self::read_response(&mut port) {
                                Ok(response) if response.trim() == "MOVE_COMPLETE" || response.trim() == "OK" => ControllerResponse::MoveComplete,
                                Ok(response) => ControllerResponse::Error(format!("Unexpected MOVE response: {}", response)),
                                Err(e) => ControllerResponse::Error(format!("MOVE response error: {}", e)),
                            }
                        }
                    }
                    ControllerCommand::SetAngle { angle } => {
                        // Формат согласно ТЗ: SETANG <deg|d:m:s>\n
                        // Используем формат d:m:s
                        let command = format!("SETANG {}", angle);
                        if let Err(e) = Self::send_command(&mut port, &command) {
                            ControllerResponse::Error(format!("SETANG failed: {}", e))
                        } else {
                            match Self::read_response(&mut port) {
                                Ok(response) if response.trim() == "ANGLE_SET" || response.trim() == "OK" => ControllerResponse::SetAngleComplete,
                                Ok(response) => ControllerResponse::Error(format!("Unexpected SETANG response: {}", response)),
                                Err(e) => ControllerResponse::Error(format!("SETANG response error: {}", e)),
                            }
                        }
                    }
                };

                // Send response back
                if resp_tx.send(response).is_err() {
                    // Channel closed, exit the thread
                    break;
                }
            }
        });

        Ok(())
    }

    /// Send a command to the controller
    fn send_command(port: &mut Box<dyn SerialPort>, command: &str) -> Result<(), String> {
        let command_with_terminator = format!("{}\n", command);
        port.write_all(command_with_terminator.as_bytes())
            .map_err(|e| e.to_string())?;
        port.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Read response from the controller
    fn read_response(port: &mut Box<dyn SerialPort>) -> Result<String, String> {
        let mut response = String::new();
        let mut buffer = [0; 1];
        
        // Read character by character until we get a newline
        loop {
            match port.read(&mut buffer) {
                Ok(1) => {
                    let ch = buffer[0] as char;
                    if ch == '\n' || ch == '\r' {
                        if !response.is_empty() {
                            break; // End of response
                        }
                    } else {
                        response.push(ch);
                    }
                }
                Ok(_) => break, // No more data to read
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    // Continue reading, this is expected for non-blocking
                    break;
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        
        Ok(response)
    }

    /// Send a command to the controller
    pub fn send_controller_command(&self, command: ControllerCommand) -> Result<(), String> {
        match &self.sender {
            Some(sender) => sender.send(command).map_err(|e| e.to_string()),
            None => Err("Not connected to controller".to_string()),
        }
    }

    /// Receive a response from the controller
    pub fn receive_response(&self) -> Result<ControllerResponse, String> {
        match &self.receiver {
            Some(receiver) => receiver.recv().map_err(|e| e.to_string()),
            None => Err("Not connected to controller".to_string()),
        }
    }

    /// Check if connected to a controller
    pub fn is_connected(&self) -> bool {
        self.sender.is_some()
    }

    /// Disconnect from the controller
    pub fn disconnect(&mut self) {
        self.sender = None;
        self.receiver = None;
        self.port_name.clear();
    }
}