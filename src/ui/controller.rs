use iced::widget::{button, column, container, pick_list, row, text, text_input};
use iced::{theme, Color, Command, Element, Length};
use std::sync::Arc;

use crate::hardware::serial_communication::{
    ControllerCommand, ControllerResponse, ControllerStatus, SerialController,
    SerialControllerConfig, SerialError,
};
use crate::utils::angle::AngleDMS;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CalibrationStep {
    NotCalibrating,
    MovingToPosition,
    Measuring,
    Complete,
}

#[derive(Debug, Clone)]
pub enum AsyncCommand {
    Ping,
    Home,
    SetZero,
    Move { x_mm: f32, speed_mm_s: f32 },
    SetAngle { angle: AngleDMS },
}

impl AsyncCommand {
    fn label(&self) -> &'static str {
        match self {
            AsyncCommand::Ping => "Pinging controller",
            AsyncCommand::Home => "Running homing sequence",
            AsyncCommand::SetZero => "Setting zero position",
            AsyncCommand::Move { .. } => "Moving stage",
            AsyncCommand::SetAngle { .. } => "Setting angle",
        }
    }

    fn to_controller_command(&self) -> ControllerCommand {
        match self {
            AsyncCommand::Ping => ControllerCommand::Ping,
            AsyncCommand::Home => ControllerCommand::Home,
            AsyncCommand::SetZero => ControllerCommand::SetZero,
            AsyncCommand::Move { x_mm, speed_mm_s } => ControllerCommand::Move {
                x_mm: *x_mm,
                speed_mm_s: *speed_mm_s,
            },
            AsyncCommand::SetAngle { angle } => ControllerCommand::SetAngle { angle: *angle },
        }
    }
}

pub struct State {
    available_ports: Vec<String>,
    selected_port: Option<String>,
    serial_config: SerialControllerConfig,
    controller: Option<Arc<SerialController>>,
    status: ControllerStatus,
    is_connecting: bool,
    command_in_flight: bool,
    status_message: String,
    last_response: Option<String>,
    position_x: f32,
    angle_dms: String,
    move_speed: f32,
    is_moving: bool,
    is_calibrating: bool,
    calibration_step: CalibrationStep,
    calibration_positions: Vec<f32>,
    current_calibration_index: usize,
}

impl State {
    pub fn new() -> Self {
        let available_ports = SerialController::detect_ports();
        let selected_port = available_ports.first().cloned();
        let status = ControllerStatus {
            port_name: String::new(),
            connected: false,
            last_error: None,
        };

        Self {
            available_ports,
            selected_port,
            serial_config: SerialControllerConfig::default(),
            controller: None,
            status,
            is_connecting: false,
            command_in_flight: false,
            status_message: "Ready".to_string(),
            last_response: None,
            position_x: 0.0,
            angle_dms: "0:0:0".into(),
            move_speed: 10.0,
            is_moving: false,
            is_calibrating: false,
            calibration_step: CalibrationStep::NotCalibrating,
            calibration_positions: vec![],
            current_calibration_index: 0,
        }
    }

    pub fn set_serial_controller(&mut self, controller: Arc<SerialController>) {
        self.status = controller.status();
        self.controller = Some(controller);
        self.is_connecting = false;
        self.status_message = format!("Connected to {}", self.status.port_name);
    }

    pub fn get_serial_controller(&self) -> Option<Arc<SerialController>> {
        self.controller.clone()
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::RefreshPorts => {
                self.status_message = "Scanning ports...".into();
                Command::perform(
                    async { SerialController::detect_ports() },
                    Message::PortsDetected,
                )
            }
            Message::PortsDetected(ports) => {
                self.available_ports = ports.clone();
                if self.available_ports.is_empty() {
                    self.selected_port = None;
                    self.status_message = "No ports detected".into();
                } else if self.selected_port.is_none() {
                    self.selected_port = self.available_ports.first().cloned();
                    self.status_message = "Ports list updated".into();
                } else {
                    self.status_message = "Ports list updated".into();
                }
                Command::none()
            }
            Message::SelectPort(port) => {
                self.selected_port = Some(port);
                Command::none()
            }
            Message::ConnectPressed => {
                if self.controller.is_some() || self.is_connecting {
                    self.status_message = "Controller already connected".into();
                    return Command::none();
                }

                if let Some(port) = self.selected_port.clone() {
                    self.is_connecting = true;
                    self.status_message = format!("Connecting to {}...", port);
                    let config = self.serial_config.clone();
                    Command::perform(connect_async(port, config), Message::ConnectFinished)
                } else {
                    self.status_message = "Select a COM port first".into();
                    Command::none()
                }
            }
            Message::ConnectFinished(result) => {
                self.is_connecting = false;
                match result {
                    Ok(controller) => {
                        self.status = controller.status();
                        self.status_message = format!("Connected to {}", self.status.port_name);
                        self.controller = Some(controller);
                    }
                    Err(err) => {
                        self.status_message = format!("Connection failed: {}", err);
                    }
                }
                Command::none()
            }
            Message::DisconnectPressed => {
                if let Some(controller) = self.controller.take() {
                    drop(controller);
                }
                self.status = ControllerStatus {
                    port_name: String::new(),
                    connected: false,
                    last_error: None,
                };
                self.status_message = "Disconnected".into();
                self.command_in_flight = false;
                Command::none()
            }
            Message::PingPressed => self.enqueue_command(AsyncCommand::Ping),
            Message::HomePressed => self.enqueue_command(AsyncCommand::Home),
            Message::SetZeroPressed => self.enqueue_command(AsyncCommand::SetZero),
            Message::MovePressed => {
                let x = self.position_x;
                let speed = self.move_speed;
                self.enqueue_command(AsyncCommand::Move {
                    x_mm: x,
                    speed_mm_s: speed,
                })
            }
            Message::SetAnglePressed => match AngleDMS::from_str_flexible(&self.angle_dms) {
                Ok(angle) => self.enqueue_command(AsyncCommand::SetAngle { angle }),
                Err(_) => {
                    self.status_message = format!("Invalid angle format: {}", self.angle_dms);
                    Command::none()
                }
            },
            Message::CommandFinished(action, result) => {
                self.command_in_flight = false;
                self.handle_command_result(action, result);
                Command::none()
            }
            Message::UpdatePositionX(value) => {
                if let Ok(x) = value.parse::<f32>() {
                    self.position_x = x;
                }
                Command::none()
            }
            Message::UpdateAngle(value) => {
                self.angle_dms = value;
                Command::none()
            }
            Message::UpdateSpeed(value) => {
                if let Ok(speed) = value.parse::<f32>() {
                    self.move_speed = speed.clamp(0.1, 100.0);
                }
                Command::none()
            }
            Message::StartCalibration => self.start_calibration(),
            Message::NextCalibrationStep => self.advance_calibration(),
        }
    }

    fn enqueue_command(&mut self, action: AsyncCommand) -> Command<Message> {
        if self.command_in_flight {
            self.status_message = "Command already in progress".into();
            return Command::none();
        }

        if let Some(controller) = self.controller.clone() {
            self.command_in_flight = true;
            self.status_message = format!("{}...", action.label());
            Command::perform(
                execute_async_command(controller, action.clone()),
                |(action, result)| Message::CommandFinished(action, result),
            )
        } else {
            self.status_message = "Controller not connected".into();
            Command::none()
        }
    }

    fn handle_command_result(
        &mut self,
        action: AsyncCommand,
        result: Result<ControllerResponse, SerialError>,
    ) {
        match result {
            Ok(response) => {
                self.last_response = Some(format!("{:?}", response));
                match action {
                    AsyncCommand::Ping => {
                        self.status_message = "PONG received".into();
                    }
                    AsyncCommand::Home => {
                        self.status_message = "Homing complete".into();
                    }
                    AsyncCommand::SetZero => {
                        self.status_message = "Zero position set".into();
                    }
                    AsyncCommand::Move { x_mm, .. } => {
                        self.position_x = x_mm;
                        self.is_moving = false;
                        self.status_message = format!("Move complete: X={:.2} mm", x_mm);
                        if self.is_calibrating {
                            self.calibration_step = CalibrationStep::Measuring;
                            self.status_message = format!(
                                "Measuring at {:.2} mm ({}/{})",
                                x_mm,
                                self.current_calibration_index + 1,
                                self.calibration_positions.len()
                            );
                        }
                    }
                    AsyncCommand::SetAngle { angle } => {
                        self.angle_dms = angle.to_string();
                        self.status_message = format!("Angle set to {}", self.angle_dms);
                    }
                }
            }
            Err(err) => {
                self.last_response = None;
                self.status_message = format!("Command failed: {}", err);
                self.is_moving = false;
            }
        }

        if let Some(controller) = self.controller.as_ref() {
            self.status = controller.status();
        } else {
            self.status = ControllerStatus {
                port_name: String::new(),
                connected: false,
                last_error: None,
            };
        }
    }

    fn start_calibration(&mut self) -> Command<Message> {
        if self.controller.is_none() {
            self.status_message = "Connect controller before calibration".into();
            return Command::none();
        }

        self.is_calibrating = true;
        self.calibration_step = CalibrationStep::MovingToPosition;
        self.calibration_positions = self.generate_calibration_positions();
        self.current_calibration_index = 0;

        if let Some(&x) = self.calibration_positions.first() {
            self.position_x = x;
            self.status_message = format!("Moving to calibration point {:.2} mm", x);
            return self.enqueue_command(AsyncCommand::Move {
                x_mm: x,
                speed_mm_s: self.move_speed,
            });
        }

        self.is_calibrating = false;
        self.calibration_step = CalibrationStep::NotCalibrating;
        self.status_message = "No calibration positions generated".into();
        Command::none()
    }

    fn advance_calibration(&mut self) -> Command<Message> {
        if !self.is_calibrating {
            return Command::none();
        }

        match self.calibration_step {
            CalibrationStep::Measuring => {
                self.current_calibration_index += 1;
                if let Some(&x) = self
                    .calibration_positions
                    .get(self.current_calibration_index)
                {
                    self.calibration_step = CalibrationStep::MovingToPosition;
                    self.position_x = x;
                    self.status_message = format!("Moving to calibration point {:.2} mm", x);
                    self.enqueue_command(AsyncCommand::Move {
                        x_mm: x,
                        speed_mm_s: self.move_speed,
                    })
                } else {
                    self.is_calibrating = false;
                    self.calibration_step = CalibrationStep::Complete;
                    self.status_message = "Calibration complete".into();
                    Command::none()
                }
            }
            CalibrationStep::Complete | CalibrationStep::NotCalibrating => Command::none(),
            CalibrationStep::MovingToPosition => {
                self.status_message = "Wait for move to complete".into();
                Command::none()
            }
        }
    }

    fn generate_calibration_positions(&self) -> Vec<f32> {
        let mut positions = Vec::new();
        let step_size = 0.2;
        let range = 10.0;

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
    RefreshPorts,
    PortsDetected(Vec<String>),
    SelectPort(String),
    ConnectPressed,
    ConnectFinished(Result<Arc<SerialController>, SerialError>),
    DisconnectPressed,
    PingPressed,
    HomePressed,
    SetZeroPressed,
    MovePressed,
    SetAnglePressed,
    CommandFinished(AsyncCommand, Result<ControllerResponse, SerialError>),
    UpdatePositionX(String),
    UpdateAngle(String),
    UpdateSpeed(String),
    StartCalibration,
    NextCalibrationStep,
}

pub fn view(state: &State) -> Element<'_, Message> {
    let port_selector: Element<Message> = if !state.available_ports.is_empty() {
        pick_list(
            state.available_ports.clone(),
            state.selected_port.clone(),
            Message::SelectPort,
        )
        .width(Length::Fixed(200.0))
        .into()
    } else {
        text("No ports available").into()
    };

    let connect_section = column![
        text("Serial Port Control").size(24),
        row![
            container(port_selector).width(Length::Shrink),
            button("Refresh")
                .on_press(Message::RefreshPorts)
                .style(theme::Button::Secondary),
            if state.controller.is_some() {
                button("Disconnect").on_press(Message::DisconnectPressed)
            } else if state.is_connecting {
                button("Connecting...")
            } else {
                button("Connect").on_press(Message::ConnectPressed)
            },
        ]
        .spacing(10),
        text(format!(
            "Status: {} ({})",
            if state.status.connected {
                "Connected"
            } else {
                "Disconnected"
            },
            state.status.port_name
        )),
        if let Some(error) = state.status.last_error.as_ref() {
            text(format!("Last error: {}", error))
                .style(theme::Text::Color(Color::from_rgb(0.9, 0.2, 0.2)))
        } else {
            text(" ")
        },
        text(&state.status_message),
        if let Some(response) = state.last_response.as_ref() {
            text(format!("Last response: {}", response))
        } else {
            text(" ")
        },
    ]
    .spacing(8)
    .padding(10);

    let motion_controls = column![
        text("Mirror Position Control").size(24),
        row![
            text_input("X (mm)", &format!("{:.2}", state.position_x))
                .on_input(Message::UpdatePositionX)
                .width(Length::Fixed(120.0)),
            text_input("Speed (mm/s)", &format!("{:.2}", state.move_speed))
                .on_input(Message::UpdateSpeed)
                .width(Length::Fixed(120.0)),
            button("Move X")
                .on_press(Message::MovePressed)
                .style(theme::Button::Primary),
        ]
        .spacing(10),
        row![
            text_input("Angle (d:m:s)", &state.angle_dms)
                .on_input(Message::UpdateAngle)
                .width(Length::Fixed(160.0)),
            button("Set Angle").on_press(Message::SetAnglePressed),
        ]
        .spacing(10),
        row![
            button("Home").on_press(Message::HomePressed),
            button("Set Zero").on_press(Message::SetZeroPressed),
            button("Ping").on_press(Message::PingPressed),
        ]
        .spacing(10),
    ]
    .spacing(8)
    .padding(10);

    let calibration_controls = column![
        text("Calibration").size(24),
        row![
            button("Start Calibration").on_press(Message::StartCalibration),
            button("Next Step").on_press(Message::NextCalibrationStep),
        ]
        .spacing(10),
        text(format!(
            "Step: {:?} ({}/{})",
            state.calibration_step,
            state.current_calibration_index
                + if state.calibration_step != CalibrationStep::NotCalibrating {
                    1
                } else {
                    0
                },
            state.calibration_positions.len()
        )),
    ]
    .spacing(8)
    .padding(10);

    column![connect_section, motion_controls, calibration_controls]
        .spacing(16)
        .padding(16)
        .into()
}

async fn connect_async(
    port: String,
    config: SerialControllerConfig,
) -> Result<Arc<SerialController>, SerialError> {
    let controller = SerialController::connect(port, config).await?;
    Ok(Arc::new(controller))
}

async fn execute_async_command(
    controller: Arc<SerialController>,
    action: AsyncCommand,
) -> (AsyncCommand, Result<ControllerResponse, SerialError>) {
    let command = action.to_controller_command();
    let result = controller.send_command(command).await;
    (action, result)
}
