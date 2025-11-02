// Система сообщений Iced для приложения

use crate::hardware::serial_communication::SerialController;
use crate::ui::camera;
use crate::ui::controller;
use crate::ui::debug;
use crate::ui::scanning;
use crate::ui::settings;
use crate::ui::visualization;
use std::sync::Arc;

/// Сообщения, которые приложение может обрабатывать
#[derive(Debug, Clone)]
pub enum Message {
    TabSelected(super::Tab),
    Controller(controller::Message),
    Camera(camera::Message),
    Scanning(scanning::Message),
    Visualization(visualization::Message),
    Settings(settings::Message),
    Debug(debug::Message),
    /// Сообщение для совместного использования serial controller между модулями
    SerialControllerConnected(Arc<SerialController>),
    SerialControllerDisconnected,
}
