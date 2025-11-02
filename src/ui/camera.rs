use iced::widget::{button, checkbox, column, row, text, text_input};
use iced::{Command, Element, Length};
use std::fmt;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

use crate::hardware::camera::{
    BackendKind, CameraError, CameraFrame, CameraMode, CameraService, FastModeSettings,
    PreciseModeSettings,
};

pub struct State {
    camera_index: u16,
    instance_name: String,
    camera: Option<Arc<CameraService>>,
    is_connecting: bool,
    streaming_enabled: bool,
    fast_mode: bool,
    status_message: String,
    last_frame: Option<CameraFrame>,
    stream_interval_ms: u64,
    capture_timeout_ms: u16,
    dimensions: Option<(i32, i32, i32)>,
}

impl State {
    pub fn new() -> Self {
        Self {
            camera_index: 0,
            instance_name: String::new(),
            camera: None,
            is_connecting: false,
            streaming_enabled: true,
            fast_mode: true,
            status_message: "Ready".into(),
            last_frame: None,
            stream_interval_ms: 150,
            capture_timeout_ms: 150,
            dimensions: None,
        }
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::UpdateCameraIndex(value) => {
                if let Ok(idx) = value.parse::<u16>() {
                    self.camera_index = idx;
                }
                Command::none()
            }
            Message::UpdateInstanceName(value) => {
                self.instance_name = value;
                Command::none()
            }
            Message::ConnectPressed => {
                if self.camera.is_some() || self.is_connecting {
                    self.status_message = "Camera already connected".into();
                    return Command::none();
                }

                self.is_connecting = true;
                self.status_message = format!("Connecting to camera {}...", self.camera_index);
                let backend = default_backend();
                let index = self.camera_index;
                let instance = if self.instance_name.trim().is_empty() {
                    None
                } else {
                    Some(self.instance_name.trim().to_string())
                };

                Command::perform(
                    connect_camera(index, backend, instance),
                    Message::ConnectFinished,
                )
            }
            Message::ConnectFinished(result) => {
                self.is_connecting = false;
                match result {
                    Ok(service) => {
                        let dims = service.dimensions();
                        self.dimensions = Some((dims.width, dims.height, dims.depth));
                        self.status_message = format!(
                            "Camera connected ({}x{}x{})",
                            dims.width, dims.height, dims.depth
                        );
                        self.camera = Some(service);
                        self.last_frame = None;
                        if self.streaming_enabled {
                            return self.schedule_next_capture();
                        }
                    }
                    Err(err) => {
                        self.camera = None;
                        self.status_message = format!("Camera connection failed: {}", err);
                        return Command::none();
                    }
                }
                Command::none()
            }
            Message::DisconnectPressed => {
                self.camera = None;
                self.last_frame = None;
                self.status_message = "Camera disconnected".into();
                return Command::none();
            }
            Message::ToggleFastMode(value) => {
                self.fast_mode = value;
                if self.streaming_enabled && self.camera.is_some() {
                    return self.schedule_next_capture();
                } else {
                    return Command::none();
                }
            }
            Message::ToggleStreaming(value) => {
                self.streaming_enabled = value;
                if value && self.camera.is_some() {
                    return self.schedule_next_capture();
                } else {
                    self.status_message = if value {
                        "Streaming enabled".into()
                    } else {
                        "Streaming paused".into()
                    };
                    return Command::none();
                }
            }
            Message::FrameCaptured(result) => {
                match result {
                    Ok(frame) => {
                        self.dimensions = Some((frame.width as i32, frame.height as i32, 1));
                        self.status_message = format!(
                            "Frame received ({}x{}) | {:.2} degC .. {:.2} degC",
                            frame.width, frame.height, frame.min_temp, frame.max_temp
                        );
                        self.last_frame = Some(frame);
                    }
                    Err(err) => {
                        self.status_message = format!("Frame error: {}", err);
                    }
                }

                if self.streaming_enabled && self.camera.is_some() {
                    return self.schedule_next_capture();
                }
                Command::none()
            }
        }
    }

    fn schedule_next_capture(&self) -> Command<Message> {
        if !self.streaming_enabled {
            return Command::none();
        }

        let camera = match self.camera.clone() {
            Some(camera) => camera,
            None => return Command::none(),
        };

        let mode = self.current_mode();
        let timeout = self.capture_timeout_ms;
        let interval = self.stream_interval_ms;

        Command::perform(
            capture_frame(camera, mode, timeout, interval),
            Message::FrameCaptured,
        )
    }

    fn current_mode(&self) -> CameraMode {
        if self.fast_mode {
            CameraMode::Fast(FastModeSettings::default())
        } else {
            CameraMode::Precise(PreciseModeSettings::default())
        }
    }

    pub fn camera_service(&self) -> Option<Arc<CameraService>> {
        self.camera.clone()
    }
}

#[derive(Clone)]
pub enum Message {
    UpdateCameraIndex(String),
    UpdateInstanceName(String),
    ConnectPressed,
    ConnectFinished(Result<Arc<CameraService>, CameraError>),
    DisconnectPressed,
    ToggleFastMode(bool),
    ToggleStreaming(bool),
    FrameCaptured(Result<CameraFrame, CameraError>),
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::UpdateCameraIndex(value) => {
                f.debug_tuple("UpdateCameraIndex").field(value).finish()
            }
            Message::UpdateInstanceName(value) => {
                f.debug_tuple("UpdateInstanceName").field(value).finish()
            }
            Message::ConnectPressed => f.write_str("ConnectPressed"),
            Message::ConnectFinished(result) => f
                .debug_struct("ConnectFinished")
                .field("success", &result.is_ok())
                .finish(),
            Message::DisconnectPressed => f.write_str("DisconnectPressed"),
            Message::ToggleFastMode(value) => f.debug_tuple("ToggleFastMode").field(value).finish(),
            Message::ToggleStreaming(value) => {
                f.debug_tuple("ToggleStreaming").field(value).finish()
            }
            Message::FrameCaptured(result) => f
                .debug_struct("FrameCaptured")
                .field("success", &result.is_ok())
                .finish(),
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let dims = state.dimensions.unwrap_or((0, 0, 0));
    let control_panel = column![
        text("Camera Control").size(24),
        row![
            text("Camera Index:"),
            text_input("0", &state.camera_index.to_string())
                .on_input(Message::UpdateCameraIndex)
                .width(Length::Fixed(90.0)),
        ]
        .spacing(10),
        row![
            text("Instance Name:"),
            text_input("PIXConnect", &state.instance_name)
                .on_input(Message::UpdateInstanceName)
                .width(Length::Fixed(200.0)),
        ]
        .spacing(10),
        row![
            if state.camera.is_some() {
                button("Disconnect").on_press(Message::DisconnectPressed)
            } else if state.is_connecting {
                button("Connecting...")
            } else {
                button("Connect").on_press(Message::ConnectPressed)
            },
            checkbox("Fast mode", state.fast_mode).on_toggle(Message::ToggleFastMode),
            checkbox("Auto stream", state.streaming_enabled).on_toggle(Message::ToggleStreaming),
        ]
        .spacing(10),
        text(format!("Status: {}", state.status_message)),
    ]
    .spacing(8)
    .padding(12);

    let frame_info = if let Some(frame) = state.last_frame.as_ref() {
        column![
            text(format!("Resolution: {}x{}", frame.width, frame.height)),
            text(format!("Mode: {:?}", frame.mode)),
            text(format!(
                "Temperature range: {:.2} degC .. {:.2} degC",
                frame.min_temp, frame.max_temp
            )),
            text(format!("Frame counter: {}", frame.metadata.frame_counter)),
        ]
    } else {
        column![
            text("Resolution: --"),
            text("Mode: --"),
            text("Temperature range: --"),
        ]
    };

    let diagnostics = column![
        text(format!(
            "Cached dimensions: {}x{}x{}",
            dims.0, dims.1, dims.2
        )),
        frame_info,
    ]
    .spacing(8)
    .padding(12);

    column![control_panel, diagnostics]
        .spacing(16)
        .padding(16)
        .into()
}

fn default_backend() -> BackendKind {
    if cfg!(target_os = "windows") {
        BackendKind::OptrisSdk
    } else {
        BackendKind::Mock
    }
}

async fn connect_camera(
    index: u16,
    backend: BackendKind,
    instance: Option<String>,
) -> Result<Arc<CameraService>, CameraError> {
    CameraService::connect(index, backend, instance).await
}

async fn capture_frame(
    camera: Arc<CameraService>,
    mode: CameraMode,
    timeout_ms: u16,
    interval_ms: u64,
) -> Result<CameraFrame, CameraError> {
    if interval_ms > 0 {
        sleep(Duration::from_millis(interval_ms)).await;
    }
    camera.capture_frame(mode, timeout_ms).await
}
