// Бизнес-логика: генерация траекторий, анализ данных, калибровка

pub mod analysis;
pub mod scan_engine;
pub mod trajectory;

// Re-export for external use
pub use analysis::TemperatureMeasurement;
