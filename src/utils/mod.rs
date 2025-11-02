// Утилиты для работы с единицами измерения, углами и преобразованиями

pub mod angle;
pub mod units;
pub mod logging;
pub mod error_handling;
pub mod serialization;

// Re-export for external use  
pub use angle::AngleDMS;
pub use error_handling::{AppError, AppResult, HardwareError, HardwareResult};

