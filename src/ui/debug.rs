use iced::widget::{button, column, container, scrollable, text};
use iced::{Element, Length, theme};

pub struct State {
    log_messages: Vec<String>,
}

impl State {
    pub fn new() -> Self {
        Self {
            log_messages: Vec::new(),
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::AddLogMessage(msg) => {
                self.log_messages.push(msg);
                // Limit the number of stored messages to prevent memory issues
                if self.log_messages.len() > 1000 {
                    self.log_messages.remove(0);
                }
            }
            Message::ClearLog => {
                self.log_messages.clear();
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    AddLogMessage(String),
    ClearLog,
}

pub fn view(state: &State) -> Element<'_, Message> {
    let log_output: Element<_> = if state.log_messages.is_empty() {
        text("No log messages.").into()
    } else {
        scrollable(
            state.log_messages.iter().fold(column![], |col, msg| col.push(text(msg)))
        ).into()
    };

    let controls = column![
        button("Clear Log").on_press(Message::ClearLog),
        container(log_output).width(Length::Fill).height(Length::Fill).style(theme::Container::Box),
    ]
    .spacing(10)
    .padding(10);

    container(controls).into()
}