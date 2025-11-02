use iced::widget::{column, container, image, row, text, text_input};
use iced::{Element, Length, theme};

pub struct State {
    // Handle for the thermal data texture
    thermal_data_texture: Option<iced::widget::image::Handle>,
    min_temp: f32,
    max_temp: f32,
}

impl State {
    pub fn new() -> Self {
        Self {
            thermal_data_texture: None,
            min_temp: 20.0,
            max_temp: 40.0,
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::UpdateThermalData(_data) => {
                // In a real implementation, this would update the thermal texture
            }
            Message::UpdateMinTemp(temp_str) => {
                if let Ok(temp) = temp_str.parse::<f32>() {
                    self.min_temp = temp;
                }
            }
            Message::UpdateMaxTemp(temp_str) => {
                if let Ok(temp) = temp_str.parse::<f32>() {
                    self.max_temp = temp;
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    UpdateThermalData(Vec<f32>),
    UpdateMinTemp(String),
    UpdateMaxTemp(String),
}

pub fn view(state: &State) -> Element<'_, Message> {
    let controls = column![
        text("Temperature Range"),
        text_input("Min Temp", &format!("{:.2}", state.min_temp)).on_input(Message::UpdateMinTemp),
        text_input("Max Temp", &format!("{:.2}", state.max_temp)).on_input(Message::UpdateMaxTemp),
    ]
    .spacing(10)
    .padding(10);

    let thermal_map = if let Some(thermal_texture) = &state.thermal_data_texture {
        container(image(thermal_texture.clone()).width(Length::Fill).height(Length::Fill))
    } else {
        container(text("No data").size(30))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .style(theme::Container::Box)
    };

    row![controls, thermal_map].spacing(20).into()
}