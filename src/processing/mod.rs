// Бизнес-логика: генерация траекторий, анализ данных, калибровка

pub mod analysis;
pub mod trajectory;

// Re-export for external use
pub use analysis::TemperatureMeasurement;
pub use trajectory::{ScanPoint, TrajectoryGenerator};
