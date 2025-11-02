// Типизированная система ошибок для подсистем приложения

use thiserror::Error;

/// Общий тип ошибок приложения
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Hardware error: {0}")]
    Hardware(#[from] HardwareError),

    #[error("Config error: {0}")]
    Config(#[from] ConfigError),

    #[error("Processing error: {0}")]
    Processing(#[from] ProcessingError),

    #[error("Data error: {0}")]
    Data(#[from] DataError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Ошибки работы с оборудованием
#[derive(Error, Debug)]
pub enum HardwareError {
    #[error("Serial port error: {0}")]
    Serial(String),

    #[error("Camera error: {0}")]
    Camera(String),

    #[error("Controller not connected")]
    NotConnected,

    #[error("Controller timeout")]
    Timeout,

    #[error("Invalid command: {0}")]
    InvalidCommand(String),

    #[error("Calibration failed: {0}")]
    CalibrationFailed(String),
}

/// Ошибки конфигурации
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file not found: {0}")]
    NotFound(String),

    #[error("Invalid config format: {0}")]
    InvalidFormat(String),

    #[error("Config parameter missing: {0}")]
    MissingParameter(String),

    #[error("Config parameter invalid: {0} = {1}")]
    InvalidParameter(String, String),
}

/// Ошибки обработки данных
#[derive(Error, Debug)]
pub enum ProcessingError {
    #[error("Trajectory generation failed: {0}")]
    TrajectoryGeneration(String),

    #[error("Image analysis failed: {0}")]
    ImageAnalysis(String),

    #[error("Calibration failed: {0}")]
    Calibration(String),

    #[error("Invalid angle format: {0}")]
    InvalidAngleFormat(String),
}

/// Ошибки работы с данными
#[derive(Error, Debug)]
pub enum DataError {
    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Export error: {0}")]
    Export(String),

    #[error("Data format error: {0}")]
    Format(String),
}

/// Тип результата для приложения
pub type AppResult<T> = Result<T, AppError>;

/// Тип результата для оборудования
pub type HardwareResult<T> = Result<T, HardwareError>;

/// Тип результата для конфигурации
pub type ConfigResult<T> = Result<T, ConfigError>;

/// Тип результата для обработки
pub type ProcessingResult<T> = Result<T, ProcessingError>;

/// Тип результата для данных
pub type DataResult<T> = Result<T, DataError>;
