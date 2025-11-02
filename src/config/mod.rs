// Управление конфигурацией: загрузка, сохранение, hot-reload INI файлов

pub mod manager;
pub mod models;

// Public API surface
pub use manager::ConfigManager;
