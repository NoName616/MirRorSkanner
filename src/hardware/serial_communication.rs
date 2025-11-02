use crate::utils::angle::AngleDMS;
use serialport::{available_ports, SerialPort, SerialPortType};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot, watch};

#[derive(Debug, Clone)]
pub enum ControllerCommand {
    Ping,
    GetId,
    Home,
    SetZero,
    /// Перемещение по оси X с указанием скорости (MOVE X=<mm> F=<mm/s>)
    Move {
        x_mm: f32,
        speed_mm_s: f32,
    },
    /// Установка угла через энкодер (SETANG <deg|d:m:s>)
    SetAngle {
        angle: AngleDMS,
    },
    /// Отправка произвольной команды, используется в отладочном режиме
    Raw {
        command: String,
    },
}

#[derive(Debug, Clone)]
pub enum ControllerResponse {
    Pong,
    Id(String),
    HomeComplete,
    SetZeroComplete,
    MoveComplete,
    SetAngleComplete,
    Raw(String),
    Error(String),
}

#[derive(Debug, Clone)]
pub struct SerialControllerConfig {
    pub baud_rate: u32,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
    pub retries: usize,
}

impl Default for SerialControllerConfig {
    fn default() -> Self {
        Self {
            baud_rate: 115_200,
            read_timeout: Duration::from_millis(500),
            write_timeout: Duration::from_millis(200),
            retries: 2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ControllerStatus {
    pub port_name: String,
    pub connected: bool,
    pub last_error: Option<String>,
}

#[derive(Debug, Error, Clone)]
pub enum SerialError {
    #[error("Serial port {0} not available")]
    PortUnavailable(String),
    #[error("Serial IO error: {0}")]
    Io(String),
    #[error("Controller timeout")]
    Timeout,
    #[error("Unexpected controller response: {0}")]
    InvalidResponse(String),
    #[error("Controller channel closed")]
    ChannelClosed,
    #[error("Failed to spawn controller worker: {0}")]
    ThreadSpawn(String),
}

impl From<serialport::Error> for SerialError {
    fn from(value: serialport::Error) -> Self {
        match value.kind() {
            serialport::ErrorKind::NoDevice => SerialError::PortUnavailable(value.to_string()),
            serialport::ErrorKind::Io(std::io::ErrorKind::TimedOut) => SerialError::Timeout,
            serialport::ErrorKind::Io(kind) => SerialError::Io(format!("{:?}", kind)),
            _ => SerialError::Io(value.to_string()),
        }
    }
}

impl From<std::io::Error> for SerialError {
    fn from(value: std::io::Error) -> Self {
        if value.kind() == std::io::ErrorKind::TimedOut {
            SerialError::Timeout
        } else {
            SerialError::Io(value.to_string())
        }
    }
}

#[derive(Debug)]
pub struct SerialController {
    port_name: String,
    config: SerialControllerConfig,
    command_tx: mpsc::UnboundedSender<WorkerMessage>,
    status_rx: watch::Receiver<ControllerStatus>,
    join_handle: Option<std::thread::JoinHandle<()>>,
}

impl SerialController {
    /// Auto-detect available COM ports, filtering out virtual ports
    pub fn detect_ports() -> Vec<String> {
        match available_ports() {
            Ok(ports) => ports
                .into_iter()
                .filter(|port| match &port.port_type {
                    SerialPortType::UsbPort(_) | SerialPortType::PciPort => true,
                    SerialPortType::BluetoothPort => false,
                    SerialPortType::Unknown => {
                        !port.port_name.contains("Virtual")
                            && !port.port_name.contains("COM_MAP")
                            && !port.port_name.contains("VCP")
                    }
                })
                .map(|port| port.port_name)
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Establishes an asynchronous controller connection.
    pub async fn connect(
        port_name: impl Into<String>,
        config: SerialControllerConfig,
    ) -> Result<Self, SerialError> {
        let port_name = port_name.into();
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let initial_status = ControllerStatus {
            port_name: port_name.clone(),
            connected: false,
            last_error: None,
        };
        let (status_tx, status_rx) = watch::channel(initial_status);
        let (handshake_tx, handshake_rx) = oneshot::channel();

        let worker_port = port_name.clone();
        let worker_config = config.clone();
        let worker = std::thread::Builder::new()
            .name(format!("serial-controller-{}", worker_port))
            .spawn(move || {
                worker_loop(worker_port, worker_config, cmd_rx, status_tx, handshake_tx);
            })
            .map_err(|e| SerialError::ThreadSpawn(e.to_string()))?;

        match handshake_rx.await {
            Ok(Ok(())) => Ok(Self {
                port_name,
                config,
                command_tx: cmd_tx,
                status_rx,
                join_handle: Some(worker),
            }),
            Ok(Err(err)) => {
                let _ = worker.join();
                Err(err)
            }
            Err(_) => {
                let _ = worker.join();
                Err(SerialError::ChannelClosed)
            }
        }
    }

    /// Sends a command asynchronously and waits for the controller response.
    pub async fn send_command(
        &self,
        command: ControllerCommand,
    ) -> Result<ControllerResponse, SerialError> {
        let (tx, rx) = oneshot::channel();
        let envelope = CommandEnvelope {
            command,
            responder: tx,
        };
        self.command_tx
            .send(WorkerMessage::Command(envelope))
            .map_err(|_| SerialError::ChannelClosed)?;

        rx.await.map_err(|_| SerialError::ChannelClosed)?
    }

    /// Returns the latest status snapshot.
    pub fn status(&self) -> ControllerStatus {
        self.status_rx.borrow().clone()
    }

    /// Subscribes to status updates.
    pub fn subscribe_status(&self) -> watch::Receiver<ControllerStatus> {
        self.status_rx.clone()
    }

    /// Initiates a graceful shutdown of the controller worker.
    pub async fn disconnect(&mut self) {
        let _ = self.command_tx.send(WorkerMessage::Shutdown);
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for SerialController {
    fn drop(&mut self) {
        let _ = self.command_tx.send(WorkerMessage::Shutdown);
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

enum WorkerMessage {
    Command(CommandEnvelope),
    Shutdown,
}

struct CommandEnvelope {
    command: ControllerCommand,
    responder: oneshot::Sender<Result<ControllerResponse, SerialError>>,
}

fn worker_loop(
    port_name: String,
    config: SerialControllerConfig,
    mut cmd_rx: mpsc::UnboundedReceiver<WorkerMessage>,
    status_tx: watch::Sender<ControllerStatus>,
    handshake_tx: oneshot::Sender<Result<(), SerialError>>,
) {
    let mut status = ControllerStatus {
        port_name: port_name.clone(),
        connected: false,
        last_error: None,
    };

    match open_port(&port_name, &config) {
        Ok(mut port) => {
            status.connected = true;
            let _ = status_tx.send(status.clone());
            let _ = handshake_tx.send(Ok(()));
            process_commands(port.as_mut(), config, &mut cmd_rx, status, status_tx);
        }
        Err(err) => {
            status.last_error = Some(err.to_string());
            let _ = status_tx.send(status);
            let _ = handshake_tx.send(Err(err));
        }
    }
}

fn open_port(
    port_name: &str,
    config: &SerialControllerConfig,
) -> Result<Box<dyn SerialPort>, SerialError> {
    let mut builder = serialport::new(port_name, config.baud_rate);
    builder = builder.timeout(config.read_timeout);
    let mut port = builder.open().map_err(SerialError::from)?;
    port.set_timeout(config.read_timeout)
        .map_err(SerialError::from)?;
    Ok(port)
}

fn process_commands(
    port: &mut dyn SerialPort,
    config: SerialControllerConfig,
    cmd_rx: &mut mpsc::UnboundedReceiver<WorkerMessage>,
    mut status: ControllerStatus,
    status_tx: watch::Sender<ControllerStatus>,
) {
    while let Some(message) = cmd_rx.blocking_recv() {
        match message {
            WorkerMessage::Shutdown => break,
            WorkerMessage::Command(CommandEnvelope { command, responder }) => {
                let result =
                    run_with_retries(&config, || execute_command(port, &config, command.clone()));

                match &result {
                    Ok(_) => status.last_error = None,
                    Err(err) => status.last_error = Some(err.to_string()),
                }

                let _ = status_tx.send(status.clone());
                let _ = responder.send(result);
            }
        }
    }

    status.connected = false;
    let _ = status_tx.send(status);
}

fn run_with_retries<F>(
    config: &SerialControllerConfig,
    mut task: F,
) -> Result<ControllerResponse, SerialError>
where
    F: FnMut() -> Result<ControllerResponse, SerialError>,
{
    let mut attempts = 0;
    loop {
        match task() {
            Ok(response) => return Ok(response),
            Err(err @ SerialError::PortUnavailable(_)) => return Err(err),
            Err(err @ SerialError::ChannelClosed) => return Err(err),
            Err(err) => {
                attempts += 1;
                if attempts > config.retries {
                    return Err(err);
                }
            }
        }
    }
}

fn execute_command(
    port: &mut dyn SerialPort,
    config: &SerialControllerConfig,
    command: ControllerCommand,
) -> Result<ControllerResponse, SerialError> {
    match command {
        ControllerCommand::Ping => {
            send_line(port, "PING", config.write_timeout)?;
            let response = read_line(port, config.read_timeout)?;
            if response.trim().eq_ignore_ascii_case("PONG") {
                Ok(ControllerResponse::Pong)
            } else {
                Ok(ControllerResponse::Error(format!(
                    "Unexpected PING response: {}",
                    response
                )))
            }
        }
        ControllerCommand::GetId => {
            send_line(port, "ID?", config.write_timeout)?;
            let response = read_line(port, config.read_timeout)?;
            Ok(ControllerResponse::Id(response.trim().to_string()))
        }
        ControllerCommand::Home => {
            send_line(port, "HOME", config.write_timeout)?;
            let response = read_line(port, config.read_timeout)?;
            if response.trim().eq_ignore_ascii_case("HOMING_COMPLETE") {
                Ok(ControllerResponse::HomeComplete)
            } else {
                Ok(ControllerResponse::Error(format!(
                    "Unexpected HOME response: {}",
                    response
                )))
            }
        }
        ControllerCommand::SetZero => {
            send_line(port, "SETZERO", config.write_timeout)?;
            let response = read_line(port, config.read_timeout)?;
            if response.trim().eq_ignore_ascii_case("ZERO_SET") {
                Ok(ControllerResponse::SetZeroComplete)
            } else {
                Ok(ControllerResponse::Error(format!(
                    "Unexpected SETZERO response: {}",
                    response
                )))
            }
        }
        ControllerCommand::Move { x_mm, speed_mm_s } => {
            let line = format!("MOVE X={:.3} F={:.3}", x_mm, speed_mm_s);
            send_line(port, &line, config.write_timeout)?;
            let response = read_line(port, config.read_timeout)?;
            if matches_response(&response, &["MOVE_COMPLETE", "OK"]) {
                Ok(ControllerResponse::MoveComplete)
            } else {
                Ok(ControllerResponse::Error(format!(
                    "Unexpected MOVE response: {}",
                    response
                )))
            }
        }
        ControllerCommand::SetAngle { angle } => {
            let line = format!("SETANG {}", angle);
            send_line(port, &line, config.write_timeout)?;
            let response = read_line(port, config.read_timeout)?;
            if matches_response(&response, &["ANGLE_SET", "OK"]) {
                Ok(ControllerResponse::SetAngleComplete)
            } else {
                Ok(ControllerResponse::Error(format!(
                    "Unexpected SETANG response: {}",
                    response
                )))
            }
        }
        ControllerCommand::Raw { command } => {
            send_line(port, &command, config.write_timeout)?;
            let response = read_line(port, config.read_timeout)?;
            Ok(ControllerResponse::Raw(response))
        }
    }
}

fn send_line(
    port: &mut dyn SerialPort,
    command: &str,
    write_timeout: Duration,
) -> Result<(), SerialError> {
    let deadline = Instant::now() + write_timeout;
    let mut bytes = command.as_bytes().to_vec();
    bytes.push(b'\n');

    port.write_all(&bytes).map_err(SerialError::from)?;
    port.flush().map_err(SerialError::from)?;

    if Instant::now() > deadline {
        return Err(SerialError::Timeout);
    }

    Ok(())
}

fn read_line(port: &mut dyn SerialPort, timeout: Duration) -> Result<String, SerialError> {
    let deadline = Instant::now() + timeout;
    let mut buffer = [0u8; 1];
    let mut bytes = Vec::new();

    loop {
        match port.read(&mut buffer) {
            Ok(n) if n > 0 => {
                for &b in &buffer[..n] {
                    if b == b'\n' || b == b'\r' {
                        if !bytes.is_empty() {
                            return Ok(String::from_utf8_lossy(&bytes).trim().to_string());
                        }
                    } else {
                        bytes.push(b);
                    }
                }
            }
            Ok(_) => {
                if Instant::now() >= deadline {
                    return Err(SerialError::Timeout);
                }
            }
            Err(ref err) if err.kind() == std::io::ErrorKind::TimedOut => {
                if Instant::now() >= deadline {
                    return Err(SerialError::Timeout);
                }
            }
            Err(err) => return Err(SerialError::from(err)),
        }
    }
}

fn matches_response(actual: &str, expected: &[&str]) -> bool {
    expected
        .iter()
        .any(|candidate| actual.trim().eq_ignore_ascii_case(candidate.trim()))
}
