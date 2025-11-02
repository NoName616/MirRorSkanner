// Глобальное состояние приложения

use crate::hardware::camera::CameraService;
use crate::hardware::serial_communication::SerialController;
use crate::ui::camera;
use crate::ui::controller;
use crate::ui::debug;
use crate::ui::scanning;
use crate::ui::settings;
use crate::ui::visualization;
use std::sync::Arc;

/// Глобальное состояние приложения MirrorScanner
pub struct MirrorScanner {
    pub active_tab: super::Tab,
    pub controller: controller::State,
    pub camera: camera::State,
    pub scanning: scanning::State,
    pub visualization: visualization::State,
    pub settings: settings::State,
    pub debug: debug::State,
    pub shared_serial_controller: Option<Arc<SerialController>>,
    pub shared_camera_service: Option<Arc<CameraService>>,
}

impl MirrorScanner {
    pub fn new() -> Self {
        Self {
            active_tab: super::Tab::Controller,
            controller: controller::State::new(),
            camera: camera::State::new(),
            scanning: scanning::State::new(),
            visualization: visualization::State::new(),
            settings: settings::State::new(),
            debug: debug::State::new(),
            shared_serial_controller: None,
            shared_camera_service: None,
        }
    }
}
