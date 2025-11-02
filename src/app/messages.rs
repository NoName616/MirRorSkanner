// Система сообщений Iced для приложения

use std::sync::{Arc, Mutex};
use crate::hardware::serial_communication::SerialController;
use crate::ui::controller;
use crate::ui::camera;
use crate::ui::scanning;
use crate::ui::visualization;
use crate::ui::settings;
use crate::ui::debug;

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
    SerialControllerConnected(Arc<Mutex<SerialController>>),
    SerialControllerDisconnected,
}

