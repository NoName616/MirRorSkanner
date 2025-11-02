use iced::widget::{button, column, container, text, text_input};
use iced::Element;

pub struct State {
    camera_port: String,
    controller_port: String,
    log_level: String,
}

impl State {
    pub fn new() -> Self {
        Self {
            camera_port: "/dev/ttyUSB0".to_string(),
            controller_port: "/dev/ttyACM0".to_string(),
            log_level: "Info".to_string(),
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::UpdateCameraPort(port) => {
                self.camera_port = port;
            }
            Message::UpdateControllerPort(port) => {
                self.controller_port = port;
            }
            Message::UpdateLogLevel(level) => {
                self.log_level = level;
            }
            Message::Save => {
                // In a real implementation, this would save the settings to a file
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    UpdateCameraPort(String),
    UpdateControllerPort(String),
    UpdateLogLevel(String),
    Save,
}

pub fn view(state: &State) -> Element<'_, Message> {
    let settings = column![
        text("Settings"),
        text_input("Camera Port", &state.camera_port).on_input(Message::UpdateCameraPort),
        text_input("Controller Port", &state.controller_port)
            .on_input(Message::UpdateControllerPort),
        text_input("Log Level", &state.log_level).on_input(Message::UpdateLogLevel),
        button("Save").on_press(Message::Save),
    ]
    .spacing(10)
    .padding(10);

    container(settings).into()
}
