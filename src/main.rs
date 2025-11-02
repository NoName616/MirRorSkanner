// Точка входа приложения

use crate::app::MirrorScanner;
use crate::config::ConfigManager;
use crate::utils::logging;
use iced::Application;
use std::path::PathBuf;

mod app;
mod config;
mod utils;
mod processing;
mod data;
mod ui;
mod hardware;

pub fn main() -> iced::Result {
    // Инициализируем tracing для базового логирования
    tracing_subscriber::fmt::init();
    
    tracing::info!("Starting Mirror Scanner GUI");

    // Инициализируем менеджер конфигурации
    let config_dir = PathBuf::from("./config");
    let config_manager = match ConfigManager::new(&config_dir) {
        Ok(manager) => {
            tracing::info!("Configuration loaded successfully");
            manager
        }
        Err(e) => {
            tracing::error!("Failed to load configuration: {}", e);
            return Err(iced::Error::WindowCreationFailed(format!(
                "Configuration error: {}", e
            ).into()));
        }
    };

    // Инициализируем систему логирования
    let logs_dir = config_manager.app_config()
        .read()
        .unwrap()
        .app
        .logs_dir
        .clone();
    
    if let Err(e) = logging::init_log_manager(&logs_dir) {
        tracing::error!("Failed to initialize log manager: {}", e);
    } else {
        crate::log_app!(logging::LogLevel::Info, "Logging system initialized");
    }

    // Запускаем hot-reload для конфигурации
    config_manager.start_hot_reload(5000); // Проверка каждые 5 секунд
    crate::log_app!(logging::LogLevel::Info, "Hot-reload started for configuration files");

    // Запускаем приложение Iced
    MirrorScanner::run(iced::Settings::default())
}
