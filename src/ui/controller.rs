use iced::widget::{button, column, container, text, text_input, pick_list, row};
use iced::Element;
use std::sync::{Arc, Mutex};

use crate::hardware::serial_communication::{SerialController, ControllerCommand};
use crate::utils::angle::AngleDMS;

pub struct State {
    is_connected: bool,
    position_x: f32,  // Позиция по оси X в мм
    angle_dms: String, // Угол в формате d:m:s
    move_speed: f32,  // Скорость перемещения в мм/с
    is_moving: bool,
    is_calibrating: bool,
    available_ports: Vec<String>,
    selected_port: Option<String>,
    serial_controller: Option<Arc<Mutex<SerialController>>>,
    status_message: String,
    // Calibration parameters
    calibration_step: CalibrationStep,
    calibration_positions: Vec<f32>,  // Только X позиции для калибровки
    current_calibration_index: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CalibrationStep {
    NotCalibrating,
    MovingToPosition,
    Measuring,
    Complete,
}

impl State {
    pub fn new() -> Self {
        let available_ports = SerialController::detect_ports();
        Self {
            is_connected: false,
            position_x: 0.0,
            angle_dms: "0:0:0".to_string(),
            move_speed: 10.0, // Скорость по умолчанию 10 мм/с
            is_moving: false,
            is_calibrating: false,
            available_ports: available_ports.clone(),
            selected_port: available_ports.first().cloned(),
            serial_controller: None,
            status_message: "Ready".to_string(),
            calibration_step: CalibrationStep::NotCalibrating,
            calibration_positions: vec![],
            current_calibration_index: 0,
        }
    }

    pub fn set_serial_controller(&mut self, controller: Arc<Mutex<SerialController>>) {
        self.serial_controller = Some(controller);
    }

    pub fn get_serial_controller(&self) -> Option<Arc<Mutex<SerialController>>> {
        self.serial_controller.clone()
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Connect => {
                if let Some(port) = &self.selected_port {
                    let mut controller = SerialController::new(11520); // Common baud rate for STM32
                    match controller.connect(port) {
                        Ok(()) => {
                            self.is_connected = true;
                            self.status_message = format!("Connected to {}", port);
                            
                            // Store the controller in an Arc<Mutex<>> to allow sharing between threads
                            let shared_controller = Arc::new(Mutex::new(controller));
                            self.serial_controller = Some(shared_controller.clone());
                            
                            // Send a PING command to verify connection
                            if let Some(ref controller) = self.serial_controller {
                                let _ = controller.lock().unwrap().send_controller_command(ControllerCommand::Ping);
                            }
                        }
                        Err(e) => {
                            self.status_message = format!("Connection failed: {}", e);
                        }
                    }
                }
            },
            Message::Disconnect => {
                if let Some(ref controller) = self.serial_controller {
                    controller.lock().unwrap().disconnect();
                }
                self.is_connected = false;
                self.serial_controller = None;
                self.status_message = "Disconnected".to_string();
            },
            Message::RefreshPorts => {
                self.available_ports = SerialController::detect_ports();
                if self.available_ports.is_empty() {
                    self.selected_port = None;
                } else if self.selected_port.is_none() {
                    self.selected_port = self.available_ports.first().cloned();
                }
            },
            Message::SelectPort(port) => {
                self.selected_port = Some(port);
            },
            Message::MoveX { x_mm, speed_mm_s } => {
                if self.is_connected {
                    if let Some(ref controller) = self.serial_controller {
                        let _ = controller.lock().unwrap().send_controller_command(
                            ControllerCommand::Move { x_mm, speed_mm_s }
                        );
                        self.is_moving = true;
                        self.position_x = x_mm;
                        self.move_speed = speed_mm_s;
                    }
                }
            },
            Message::SetAngle { angle_str } => {
                if self.is_connected {
                    if let Ok(angle) = AngleDMS::from_str_flexible(&angle_str) {
                        if let Some(ref controller) = self.serial_controller {
                            let _ = controller.lock().unwrap().send_controller_command(
                                ControllerCommand::SetAngle { angle }
                            );
                            self.angle_dms = angle_str.clone();
                            self.status_message = format!("Angle set to: {}", &angle_str);
                        }
                    } else {
                        self.status_message = format!("Invalid angle format: {}", &angle_str);
                    }
                }
            },
            Message::Home => {
                if self.is_connected {
                    if let Some(ref controller) = self.serial_controller {
                        let _ = controller.lock().unwrap().send_controller_command(ControllerCommand::Home);
                        self.status_message = "Homing sequence initiated".to_string();
                    }
                }
            },
            Message::SetZero => {
                if self.is_connected {
                    if let Some(ref controller) = self.serial_controller {
                        let _ = controller.lock().unwrap().send_controller_command(ControllerCommand::SetZero);
                        self.status_message = "Zero position set".to_string();
                    }
                }
            },
            Message::Ping => {
                if self.is_connected {
                    if let Some(ref controller) = self.serial_controller {
                        let _ = controller.lock().unwrap().send_controller_command(ControllerCommand::Ping);
                        self.status_message = "Ping command sent".to_string();
                    }
                }
            },
            Message::GetPosition => {
                // In a real implementation, you would query the controller for current position
                // For now, we'll just update the status
                self.status_message = format!("Current position: X={:.2} mm, Angle={}", self.position_x, self.angle_dms);
            },
            Message::UpdatePositionX(x_str) => {
                if let Ok(x) = x_str.parse::<f32>() {
                    self.position_x = x;
                }
            },
            Message::UpdateAngle(angle_str) => {
                self.angle_dms = angle_str;
            },
            Message::UpdateSpeed(speed_str) => {
                if let Ok(speed) = speed_str.parse::<f32>() {
                    self.move_speed = speed.max(0.1).min(100.0); // Ограничение скорости
                }
            },
            Message::StartCalibration => {
                if self.is_connected {
                    self.is_calibrating = true;
                    self.calibration_step = CalibrationStep::MovingToPosition;
                    self.status_message = "Starting calibration...".to_string();
                    
                    // Define calibration positions - only X axis
                    self.calibration_positions = self.generate_calibration_positions();
                    self.current_calibration_index = 0;
                    
                    if !self.calibration_positions.is_empty() {
                        let x = self.calibration_positions[0];
                        self.move_to_position(x);
                    } else {
                        self.is_calibrating = false;
                        self.calibration_step = CalibrationStep::NotCalibrating;
                        self.status_message = "No calibration positions generated".to_string();
                    }
                }
            },
            Message::NextCalibrationStep => {
                if self.is_calibrating {
                    match self.calibration_step {
                        CalibrationStep::MovingToPosition => {
                            // After moving to position, we move to measurement step
                            self.calibration_step = CalibrationStep::Measuring;
                            self.status_message = format!(
                                "Measuring at X position {:.2} mm - {} of {}",
                                self.position_x,
                                self.current_calibration_index + 1,
                                self.calibration_positions.len()
                            );
                            
                            // In a real implementation, we would trigger camera measurement here
                            // For now, we'll just move to the next position after a delay
                            
                            // Move to next position after measurement
                            self.current_calibration_index += 1;
                            if self.current_calibration_index < self.calibration_positions.len() {
                                let x = self.calibration_positions[self.current_calibration_index];
                                self.move_to_position(x);
                            } else {
                                self.calibration_step = CalibrationStep::Complete;
                                self.is_calibrating = false;
                                self.status_message = "Calibration complete".to_string();
                            }
                        },
                        CalibrationStep::Measuring => {
                            // This would be handled by actual measurement logic
                        },
                        CalibrationStep::Complete => {
                            self.is_calibrating = false;
                            self.calibration_step = CalibrationStep::NotCalibrating;
                        },
                        CalibrationStep::NotCalibrating => {
                            // Should not happen
                        }
                    }
                }
            },
        }
    }

    // Helper function to move to a specific X position
    fn move_to_position(&mut self, x_mm: f32) {
        if let Some(ref controller) = self.serial_controller {
            let _ = controller.lock().unwrap().send_controller_command(
                ControllerCommand::Move { x_mm, speed_mm_s: self.move_speed }
            );
            self.position_x = x_mm;
        }
    }

    // Generate positions for calibration - только по оси X
    fn generate_calibration_positions(&self) -> Vec<f32> {
        let mut positions = Vec::new();
        let step_size = 0.2; // 0.2mm steps for calibration
        let range = 10.0; // ±10mm range for calibration
        
        let mut x = -range;
        while x <= range {
            positions.push(x);
            x += step_size;
        }

        positions
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Connect,
    Disconnect,
    RefreshPorts,
    SelectPort(String),
    MoveX { x_mm: f32, speed_mm_s: f32 },
    SetAngle { angle_str: String },
    UpdatePositionX(String),
    UpdateAngle(String),
    UpdateSpeed(String),
    Home,
    SetZero,
    Ping,
    GetPosition,
    StartCalibration,
    NextCalibrationStep,
}

pub fn view(state: &State) -> Element<'_, Message> {
    let port_selection: Element<Message> = if !state.available_ports.is_empty() {
        pick_list(state.available_ports.clone(), state.selected_port.clone(), Message::SelectPort).into()
    } else {
        text("No ports available").into()
    };

    let connect_button = if state.is_connected {
        button("Disconnect").on_press(Message::Disconnect)
    } else {
        button("Connect").on_press(Message::Connect)
    };

    let controls = column![
        column![
            text("Serial Port Control"),
            row![
                container(port_selection).width(200), // Give it a fixed width
                button("Refresh").on_press(Message::RefreshPorts),
                connect_button,
            ].spacing(10),
        ].spacing(5),
        column![
            text("Mirror Position Control"),
            row![
                text_input("X (mm)", &format!("{:.2}", state.position_x))
                    .on_input(Message::UpdatePositionX),
                text_input("Speed (mm/s)", &format!("{:.2}", state.move_speed))
                    .on_input(Message::UpdateSpeed),
                button("Move X").on_press(Message::MoveX { 
                    x_mm: state.position_x, 
                    speed_mm_s: state.move_speed 
                }),
            ].spacing(10),
            row![
                text_input("Angle (d:m:s)", &state.angle_dms)
                    .on_input(Message::UpdateAngle),
                button("Set Angle").on_press(Message::SetAngle { 
                    angle_str: state.angle_dms.clone() 
                }),
            ].spacing(10),
        ].spacing(5),
        column![
            text("Commands"),
            row![
                button("Home").on_press(Message::Home),
                button("Set Zero").on_press(Message::SetZero),
                button("Calibrate").on_press(Message::StartCalibration),
                button("Ping").on_press(Message::Ping),
                button("Get Position").on_press(Message::GetPosition),
            ].spacing(10),
        ].spacing(5),
        text(format!("Status: {}", state.status_message)),
        text(format!("Connection: {}", if state.is_connected { "Connected" } else { "Disconnected" })),
        text(format!("Position: X={:.2} mm, Angle={}", state.position_x, state.angle_dms)),
        text(format!("Moving: {}", if state.is_moving { "Yes" } else { "No" })),
        text(format!("Calibrating: {}", if state.is_calibrating { "Yes" } else { "No" })),
    ]
    .spacing(10)
    .padding(10);

    container(controls).into()
}