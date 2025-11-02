// Хранение данных измерений и управление ими

use crate::processing::TemperatureMeasurement;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Хранилище данных измерений
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasurementStorage {
    /// Список всех измерений
    pub measurements: Vec<TemperatureMeasurement>,

    /// Метаданные сканирования
    pub metadata: ScanMetadata,

    /// Статистика измерений
    pub statistics: MeasurementStatistics,
}

/// Метаданные сканирования
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanMetadata {
    /// Дата и время начала сканирования
    pub start_time: DateTime<Local>,

    /// Дата и время окончания сканирования
    pub end_time: Option<DateTime<Local>>,

    /// Параметры сканирования
    pub scan_params: ScanParameters,

    /// Калибровка контроллера (опционально)
    pub controller_calibration: Option<String>,

    /// Калибровка камеры (опционально)
    pub camera_calibration: Option<String>,
}

/// Параметры сканирования
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanParameters {
    pub radius_mm: f64,
    pub pitch_mm: f64,
    pub angle: String,
    pub dwell_ms: u32,
    pub order: String,
}

/// Статистика измерений
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasurementStatistics {
    pub total_count: usize,
    pub min_temp: f32,
    pub max_temp: f32,
    pub avg_temp: f32,
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
}

impl MeasurementStorage {
    /// Создает новое хранилище данных
    pub fn new(scan_params: ScanParameters) -> Self {
        Self {
            measurements: Vec::new(),
            metadata: ScanMetadata {
                start_time: Local::now(),
                end_time: None,
                scan_params,
                controller_calibration: None,
                camera_calibration: None,
            },
            statistics: MeasurementStatistics {
                total_count: 0,
                min_temp: f32::MAX,
                max_temp: f32::MIN,
                avg_temp: 0.0,
                min_x: f64::MAX,
                max_x: f64::MIN,
                min_y: f64::MAX,
                max_y: f64::MIN,
            },
        }
    }

    /// Добавляет измерение в хранилище
    pub fn add_measurement(&mut self, measurement: TemperatureMeasurement) {
        self.measurements.push(measurement.clone());
        self.update_statistics(&measurement);
    }

    /// Добавляет несколько измерений
    pub fn add_measurements(&mut self, measurements: &[TemperatureMeasurement]) {
        for measurement in measurements {
            self.add_measurement(measurement.clone());
        }
    }

    /// Обновляет статистику при добавлении измерения
    fn update_statistics(&mut self, measurement: &TemperatureMeasurement) {
        self.statistics.total_count = self.measurements.len();

        // Обновляем температуры
        self.statistics.min_temp = self
            .statistics
            .min_temp
            .min(measurement.temp_min)
            .min(measurement.temp_max);
        self.statistics.max_temp = self
            .statistics
            .max_temp
            .max(measurement.temp_min)
            .max(measurement.temp_max);

        // Вычисляем среднюю температуру
        let sum: f32 = self
            .measurements
            .iter()
            .map(|m| m.temp_min + m.temp_max)
            .sum();
        let count = self.measurements.len() as f32 * 2.0;
        self.statistics.avg_temp = if count > 0.0 { sum / count } else { 0.0 };

        // Обновляем координаты
        self.statistics.min_x = self.statistics.min_x.min(measurement.x_mm);
        self.statistics.max_x = self.statistics.max_x.max(measurement.x_mm);
        self.statistics.min_y = self.statistics.min_y.min(measurement.y_mm);
        self.statistics.max_y = self.statistics.max_y.max(measurement.y_mm);
    }

    /// Завершает сканирование
    pub fn finish_scan(&mut self) {
        self.metadata.end_time = Some(Local::now());
    }

    /// Сохраняет хранилище в файл
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize: {}", e))?;

        fs::write(path, json).map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(())
    }

    /// Загружает хранилище из файла
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let contents =
            fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

        let storage: Self =
            serde_json::from_str(&contents).map_err(|e| format!("Failed to deserialize: {}", e))?;

        Ok(storage)
    }

    /// Получает измерения по фильтру
    pub fn get_measurements_filtered(
        &self,
        filter: &MeasurementFilter,
    ) -> Vec<&TemperatureMeasurement> {
        self.measurements
            .iter()
            .filter(|m| filter.matches(m))
            .collect()
    }

    /// Очищает все измерения
    pub fn clear(&mut self) {
        self.measurements.clear();
        self.statistics = MeasurementStatistics {
            total_count: 0,
            min_temp: f32::MAX,
            max_temp: f32::MIN,
            avg_temp: 0.0,
            min_x: f64::MAX,
            max_x: f64::MIN,
            min_y: f64::MAX,
            max_y: f64::MIN,
        };
    }
}

/// Фильтр для выборки измерений
#[derive(Debug, Clone)]
pub struct MeasurementFilter {
    pub min_temp: Option<f32>,
    pub max_temp: Option<f32>,
    pub min_x: Option<f64>,
    pub max_x: Option<f64>,
    pub min_y: Option<f64>,
    pub max_y: Option<f64>,
}

impl MeasurementFilter {
    /// Создает пустой фильтр (принимает все измерения)
    pub fn new() -> Self {
        Self {
            min_temp: None,
            max_temp: None,
            min_x: None,
            max_x: None,
            min_y: None,
            max_y: None,
        }
    }

    /// Проверяет, соответствует ли измерение фильтру
    pub fn matches(&self, measurement: &TemperatureMeasurement) -> bool {
        if let Some(min) = self.min_temp {
            if measurement.temp_min < min && measurement.temp_max < min {
                return false;
            }
        }

        if let Some(max) = self.max_temp {
            if measurement.temp_min > max && measurement.temp_max > max {
                return false;
            }
        }

        if let Some(min) = self.min_x {
            if measurement.x_mm < min {
                return false;
            }
        }

        if let Some(max) = self.max_x {
            if measurement.x_mm > max {
                return false;
            }
        }

        if let Some(min) = self.min_y {
            if measurement.y_mm < min {
                return false;
            }
        }

        if let Some(max) = self.max_y {
            if measurement.y_mm > max {
                return false;
            }
        }

        true
    }
}

/// Менеджер хранилищ данных (управление несколькими сканированиями)
pub struct StorageManager {
    storage_dir: PathBuf,
    active_storages: HashMap<String, MeasurementStorage>,
}

impl StorageManager {
    /// Создает новый менеджер хранилищ
    pub fn new(storage_dir: impl AsRef<Path>) -> Result<Self, String> {
        let storage_dir = storage_dir.as_ref().to_path_buf();
        fs::create_dir_all(&storage_dir)
            .map_err(|e| format!("Failed to create storage directory: {}", e))?;

        Ok(Self {
            storage_dir,
            active_storages: HashMap::new(),
        })
    }

    /// Создает новое хранилище для сканирования
    pub fn create_storage(
        &mut self,
        scan_id: String,
        scan_params: ScanParameters,
    ) -> &mut MeasurementStorage {
        let storage = MeasurementStorage::new(scan_params);
        let scan_id_clone = scan_id.clone();
        self.active_storages.insert(scan_id, storage);
        self.active_storages.get_mut(&scan_id_clone).unwrap()
    }

    /// Получает хранилище по ID
    pub fn get_storage(&mut self, scan_id: &str) -> Option<&mut MeasurementStorage> {
        self.active_storages.get_mut(scan_id)
    }

    /// Сохраняет хранилище в файл
    pub fn save_storage(&self, scan_id: &str) -> Result<(), String> {
        if let Some(storage) = self.active_storages.get(scan_id) {
            let filename = format!("scan_{}.json", scan_id);
            let path = self.storage_dir.join(filename);
            storage.save(path)
        } else {
            Err(format!("Storage with ID '{}' not found", scan_id))
        }
    }

    /// Загружает хранилище из файла
    pub fn load_storage(&mut self, scan_id: &str) -> Result<(), String> {
        let filename = format!("scan_{}.json", scan_id);
        let path = self.storage_dir.join(filename);
        let storage = MeasurementStorage::load(path)?;
        self.active_storages.insert(scan_id.to_string(), storage);
        Ok(())
    }

    /// Удаляет хранилище из памяти
    pub fn remove_storage(&mut self, scan_id: &str) {
        self.active_storages.remove(scan_id);
    }
}
