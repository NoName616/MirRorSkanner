// Управление состоянием и основная логика приложения

pub mod messages;
pub mod state;
pub mod theme;

use iced::widget::{column, container, row};
use iced::{executor, Application, Command, Element, Theme};
// UI компоненты
use crate::ui::camera;
use crate::ui::controller;
use crate::ui::debug;
use crate::ui::scanning;
use crate::ui::settings;
use crate::ui::visualization;

/// Перечисление вкладок в панели навигации
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Controller,
    Camera,
    Scanning,
    Visualization,
    Settings,
    Debug,
}

// Re-export для удобства
pub use messages::Message;
pub use state::MirrorScanner;
use theme::TabButton;

impl Application for MirrorScanner {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (MirrorScanner::new(), Command::none())
    }

    fn title(&self) -> String {
        String::from("Mirror Scanner GUI")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::TabSelected(tab) => {
                self.active_tab = tab;
                Command::none()
            }
            Message::Controller(msg) => {
                let command = self.controller.update(msg);

                if let Some(controller) = self.controller.get_serial_controller() {
                    self.controller.set_serial_controller(controller.clone());
                    self.shared_serial_controller = Some(controller.clone());
                    self.scanning.set_serial_controller(controller);
                } else {
                    self.shared_serial_controller = None;
                    self.scanning.clear_serial_controller();
                }

                command.map(Message::Controller)
            }
            Message::Camera(msg) => self.camera.update(msg).map(Message::Camera),
            Message::Scanning(msg) => {
                // Handle scanning messages
                self.scanning.update(msg);

                // If we have a shared controller, set it for the scanning module
                if let Some(ref controller) = self.shared_serial_controller {
                    self.scanning.set_serial_controller(controller.clone());
                }
                Command::none()
            }
            Message::Visualization(msg) => {
                self.visualization.update(msg);
                Command::none()
            }
            Message::Settings(msg) => {
                self.settings.update(msg);
                Command::none()
            }
            Message::Debug(msg) => {
                self.debug.update(msg);
                Command::none()
            }
            Message::SerialControllerConnected(controller) => {
                self.shared_serial_controller = Some(controller.clone());
                // Update both controller and scanning modules with the shared controller
                if let Some(ref controller) = self.shared_serial_controller {
                    self.controller.set_serial_controller(controller.clone());
                    self.scanning.set_serial_controller(controller.clone());
                }
                Command::none()
            }
            Message::SerialControllerDisconnected => {
                self.shared_serial_controller = None;
                Command::none()
            }
        }
    }

    fn theme(&self) -> Theme {
        theme::dark_theme()
    }

    fn view(&self) -> Element<'_, Message> {
        let navigation = row![
            TabButton::new("Controller", Tab::Controller, self.active_tab),
            TabButton::new("Camera", Tab::Camera, self.active_tab),
            TabButton::new("Scanning", Tab::Scanning, self.active_tab),
            TabButton::new("Visualization", Tab::Visualization, self.active_tab),
            TabButton::new("Settings", Tab::Settings, self.active_tab),
            TabButton::new("Debug", Tab::Debug, self.active_tab),
        ]
        .spacing(10);

        let content = match self.active_tab {
            Tab::Controller => controller::view(&self.controller).map(Message::Controller),
            Tab::Camera => camera::view(&self.camera).map(Message::Camera),
            Tab::Scanning => scanning::view(&self.scanning).map(Message::Scanning),
            Tab::Visualization => {
                visualization::view(&self.visualization).map(Message::Visualization)
            }
            Tab::Settings => settings::view(&self.settings).map(Message::Settings),
            Tab::Debug => debug::view(&self.debug).map(Message::Debug),
        };

        column![
            navigation,
            container(content)
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .center_x()
                .center_y(),
        ]
        .spacing(20)
        .padding(20)
        .into()
    }
}
