use crate::hardware::camera::{OptrisCamera, SharedCamera};
use anyhow::Result as AnyhowResult;
use iced::Element;
use std::sync::{Arc, Mutex};

pub struct State {
    is_connected: bool,
    camera_index: u16,
    frame_width: i32,
    frame_height: i32,
    frame_data: Vec<f32>, // Температуры в градусах Цельсия
    temperature_min: f32,
    temperature_max: f32,
    fast_mode: bool,
    camera: Option<SharedCamera>,
    status_message: String,
    instance_name: String,
}

impl State {
    pub fn new() -> Self {
        Self {
            is_connected: false,
            camera_index: 0,
            frame_width: 0,
            frame_height: 0,
            frame_data: Vec::new(),
            temperature_min: 20.0,
            temperature_max: 40.0,
            fast_mode: true,
            camera: None,
            status_message: "Ready".to_string(),
            instance_name: String::new(),
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Connect => match self.connect() {
                Ok(_) => {
                    self.is_connected = true;
                    self.status_message =
                        format!("Connected to camera index {}", self.camera_index);
                }
                Err(e) => {
                    self.status_message = format!("Connection failed: {}", e);
                    self.is_connected = false;
                }
            },
            Message::Disconnect => {
                if let Some(ref camera) = self.camera {
                    if let Ok(mut cam) = camera.lock() {
                        let _ = cam.release();
                    }
                }
                self.is_connected = false;
                self.camera = None;
                self.status_message = "Disconnected".to_string();
            }
            Message::GetFrame => {
                if let Some(ref camera) = self.camera {
                    match self.acquire_frame(camera) {
                        Ok(_) => {
                            self.status_message = "Frame acquired".to_string();
                        }
                        Err(e) => {
                            self.status_message = format!("Failed to get frame: {}", e);
                        }
                    }
                }
            }
            Message::FrameReceived(result) => match result {
                Ok((temps, min, max)) => {
                    self.frame_data = temps;
                    self.temperature_min = min;
                    self.temperature_max = max;
                }
                Err(e) => {
                    self.status_message = format!("Frame error: {}", e);
                }
            },
            Message::UpdateCameraIndex(idx_str) => {
                if let Ok(idx) = idx_str.parse::<u16>() {
                    self.camera_index = idx;
                }
            }
            Message::UpdateInstanceName(name) => {
                self.instance_name = name;
            }
            Message::ToggleFastMode(enabled) => {
                self.fast_mode = enabled;
            }
            Message::UpdateMinTemp(temp_str) => {
                if let Ok(temp) = temp_str.parse::<f32>() {
                    self.temperature_min = temp;
                }
            }
            Message::UpdateMaxTemp(temp_str) => {
                if let Ok(temp) = temp_str.parse::<f32>() {
                    self.temperature_max = temp;
                }
            }
        }
    }

    fn connect(&mut self) -> AnyhowResult<()> {
        let mut camera = OptrisCamera::new(self.camera_index);

        let result = if self.instance_name.is_empty() {
            camera.init()
        } else {
            camera.init_named(&self.instance_name)
        };

        match result {
            Ok(_) => {
                let (w, h, _d) = camera.get_frame_size();
                self.frame_width = w;
                self.frame_height = h;
                self.camera = Some(Arc::new(Mutex::new(camera)));
                Ok(())
            }
            Err(e) => Err(anyhow::anyhow!("Failed to connect camera: {}", e)),
        }
    }

    fn acquire_frame(&self, camera: &SharedCamera) -> AnyhowResult<()> {
        let mut cam = camera
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        let (raw_frame, _metadata) = cam
            .get_raw_frame(100)
            .map_err(|e| anyhow::anyhow!("Failed to get frame: {}", e))?;

        // Конвертируем в температуры
        let temperatures = cam.convert_to_temperature_celsius(&raw_frame);

        // Находим min и max
        let _min_temp = temperatures.iter().fold(f32::MAX, |a, &b| a.min(b));
        let _max_temp = temperatures.iter().fold(f32::MIN, |a, &b| a.max(b));

        // Обновляем состояние (это должно быть через сообщение)
        // Пока просто возвращаем успех

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Connect,
    Disconnect,
    GetFrame,
    FrameReceived(std::result::Result<(Vec<f32>, f32, f32), String>),
    UpdateCameraIndex(String),
    UpdateInstanceName(String),
    ToggleFastMode(bool),
    UpdateMinTemp(String),
    UpdateMaxTemp(String),
}

pub fn view(state: &State) -> Element<'_, Message> {
    use iced::widget::{button, checkbox, column, container, row, text, text_input};
    use iced::{theme, Length};

    let controls = column![
        text("Camera Control"),
        row![
            text("Camera Index:"),
            text_input("0", &format!("{}", state.camera_index))
                .on_input(Message::UpdateCameraIndex)
                .width(100),
        ]
        .spacing(10),
        row![
            text("Instance Name (optional):"),
            text_input("", &state.instance_name)
                .on_input(Message::UpdateInstanceName)
                .width(200),
        ]
        .spacing(10),
        row![
            if state.is_connected {
                button("Disconnect").on_press(Message::Disconnect)
            } else {
                button("Connect").on_press(Message::Connect)
            },
            button("Get Frame").on_press(Message::GetFrame),
        ]
        .spacing(10),
        checkbox("Fast Mode", state.fast_mode).on_toggle(Message::ToggleFastMode),
        text(format!("Status: {}", state.status_message)),
        text(format!(
            "Connection: {}",
            if state.is_connected {
                "Connected"
            } else {
                "Disconnected"
            }
        )),
        text(format!(
            "Frame: {}x{}",
            state.frame_width, state.frame_height
        )),
        text(format!(
            "Temperatures: {}°C - {}°C",
            state.temperature_min, state.temperature_max
        )),
    ]
    .spacing(10)
    .padding(10);

    let video_feed = if state.frame_width > 0 && state.frame_height > 0 {
        // В реальной реализации здесь будет отображение термического изображения
        container(
            text(format!(
                "Thermal Image\n{}x{} pixels\nT: {:.1}°C - {:.1}°C",
                state.frame_width, state.frame_height, state.temperature_min, state.temperature_max
            ))
            .size(16),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .style(theme::Container::Box)
    } else {
        // Placeholder until the first frame is received
        container(text("No signal\nConnect camera and get frame").size(30))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .style(theme::Container::Box)
    };

    row![controls, video_feed].spacing(20).into()
}
