// Управление конфигурацией: загрузка, сохранение, hot-reload INI файлов

pub mod manager;
pub mod models;

// Backward compatibility aliases
pub use manager as loader;
pub use models as types;

// Re-export для удобства
pub use manager::ConfigManager;
pub use models::*;
