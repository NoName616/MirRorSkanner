// Бизнес-логика: генерация траекторий, анализ данных, калибровка

pub mod trajectory;
pub mod analysis;

// Re-export for external use
pub use trajectory::{ScanPoint, TrajectoryGenerator};
pub use analysis::TemperatureMeasurement;

