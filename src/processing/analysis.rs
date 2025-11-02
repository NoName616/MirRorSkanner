// Анализ термических изображений, анти-дублирование измерений

use std::hash::{Hash, Hasher};
use std::time::{Duration, SystemTime};

/// Структура для хранения температурного измерения
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TemperatureMeasurement {
    pub x_mm: f64,
    pub y_mm: f64,
    pub angle_dms: String,
    pub temp_min: f32,
    pub temp_max: f32,
    pub timestamp: SystemTime,
    pub pixel_x: u32,
    pub pixel_y: u32,
}

impl Hash for TemperatureMeasurement {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Хэшируем только пространственные координаты и температуру
        (self.pixel_x, self.pixel_y).hash(state);
        (self.temp_min as u32, self.temp_max as u32).hash(state);
    }
}

impl Eq for TemperatureMeasurement {}

/// Система анти-дублирования измерений
pub struct DeduplicationSystem {
    /// Пространственный радиус фильтрации (в пикселях)
    spatial_radius: f64,
    /// Временное окно фильтрации (в миллисекундах)
    time_window_ms: u64,
    /// История измерений для проверки дубликатов
    history: Vec<(TemperatureMeasurement, SystemTime)>,
    /// Максимальный размер истории
    max_history_size: usize,
}

impl DeduplicationSystem {
    /// Создает новую систему анти-дублирования
    pub fn new(spatial_radius: f64, time_window_ms: u64) -> Self {
        Self {
            spatial_radius,
            time_window_ms,
            history: Vec::new(),
            max_history_size: 1000,
        }
    }

    /// Проверяет, является ли измерение дубликатом
    pub fn is_duplicate(&self, measurement: &TemperatureMeasurement) -> bool {
        let now = SystemTime::now();
        let time_threshold = Duration::from_millis(self.time_window_ms);

        // Удаляем старые записи
        let recent_history: Vec<_> = self
            .history
            .iter()
            .filter(|(_, timestamp)| {
                now.duration_since(*timestamp).unwrap_or(Duration::ZERO) < time_threshold
            })
            .collect();

        // Проверяем пространственное совпадение
        for (hist_measurement, _) in recent_history {
            let dx = hist_measurement.pixel_x as f64 - measurement.pixel_x as f64;
            let dy = hist_measurement.pixel_y as f64 - measurement.pixel_y as f64;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance <= self.spatial_radius {
                // Проверяем температурное совпадение
                let temp_diff_min = (hist_measurement.temp_min - measurement.temp_min).abs();
                let temp_diff_max = (hist_measurement.temp_max - measurement.temp_max).abs();

                if temp_diff_min < 0.1 && temp_diff_max < 0.1 {
                    return true; // Дубликат найден
                }
            }
        }

        false
    }

    /// Добавляет измерение в историю
    pub fn add_measurement(&mut self, measurement: TemperatureMeasurement) {
        let now = SystemTime::now();

        // Очищаем старые записи
        self.history.retain(|(_, timestamp)| {
            now.duration_since(*timestamp).unwrap_or(Duration::ZERO)
                < Duration::from_millis(self.time_window_ms)
        });

        // Добавляем новое измерение
        self.history.push((measurement, now));

        // Ограничиваем размер истории
        if self.history.len() > self.max_history_size {
            self.history.remove(0);
        }
    }

    /// Проверяет и добавляет измерение, возвращая true если это не дубликат
    pub fn check_and_add(&mut self, measurement: TemperatureMeasurement) -> bool {
        if self.is_duplicate(&measurement) {
            false
        } else {
            self.add_measurement(measurement.clone());
            true
        }
    }
}

/// Быстрый режим анализа изображения (downsampling 4:1, ROI)
pub struct FastImageAnalyzer {
    downscale_factor: u32,
}

impl FastImageAnalyzer {
    pub fn new() -> Self {
        Self {
            downscale_factor: 4,
        }
    }

    /// Создает быстрый анализатор с заданным коэффициентом даунсэмплинга
    pub fn with_downscale_factor(factor: u32) -> Self {
        let factor = factor.max(1);
        Self {
            downscale_factor: factor,
        }
    }

    /// Возвращает текущий коэффициент даунсэмплинга
    pub fn downscale_factor(&self) -> u32 {
        self.downscale_factor
    }

    /// Анализирует изображение в быстром режиме
    /// Возвращает минимальную и максимальную температуру из ROI
    pub fn analyze_fast(
        &self,
        image_data: &[f32],
        width: u32,
        height: u32,
        roi: Option<(u32, u32, u32, u32)>,
    ) -> (f32, f32) {
        let (roi_x, roi_y, roi_w, roi_h) = roi.unwrap_or((0, 0, width, height));

        let mut min_temp = f32::MAX;
        let mut max_temp = f32::MIN;

        // Обрабатываем только ROI с даунсэмплингом
        for y in (roi_y..roi_y + roi_h).step_by(self.downscale_factor as usize) {
            for x in (roi_x..roi_x + roi_w).step_by(self.downscale_factor as usize) {
                let idx = (y * width + x) as usize;
                if idx < image_data.len() {
                    let temp = image_data[idx];
                    min_temp = min_temp.min(temp);
                    max_temp = max_temp.max(temp);
                }
            }
        }

        (min_temp, max_temp)
    }
}

/// Точный режим анализа изображения (полнокадровая обработка, медианная фильтрация)
pub struct PreciseImageAnalyzer {
    median_kernel_size: u32,
}

impl PreciseImageAnalyzer {
    pub fn new() -> Self {
        Self {
            median_kernel_size: 3,
        }
    }

    /// Создает точный анализатор с заданным размером медианного ядра
    pub fn with_kernel_size(size: u32) -> Self {
        let size = if size % 2 == 0 { size + 1 } else { size }.max(1);
        Self {
            median_kernel_size: size,
        }
    }

    /// Возвращает размер медианного ядра
    pub fn kernel_size(&self) -> u32 {
        self.median_kernel_size
    }

    /// Анализирует изображение в точном режиме
    pub fn analyze_precise(&self, image_data: &[f32], width: u32, height: u32) -> (f32, f32) {
        // Применяем медианную фильтрацию
        let filtered = self.apply_median_filter(image_data, width, height);

        let min_temp = filtered.iter().copied().fold(f32::MAX, f32::min);
        let max_temp = filtered.iter().copied().fold(f32::MIN, f32::max);

        (min_temp, max_temp)
    }

    /// Применяет медианную фильтрацию
    fn apply_median_filter(&self, image_data: &[f32], width: u32, height: u32) -> Vec<f32> {
        let mut filtered = vec![0.0f32; image_data.len()];
        let kernel_half = (self.median_kernel_size / 2) as i32;

        for y in 0..height as i32 {
            for x in 0..width as i32 {
                let mut values = Vec::new();

                for ky in -kernel_half..=kernel_half {
                    for kx in -kernel_half..=kernel_half {
                        let px = x + kx;
                        let py = y + ky;

                        if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                            let idx = (py * width as i32 + px) as usize;
                            if idx < image_data.len() {
                                values.push(image_data[idx]);
                            }
                        }
                    }
                }

                let idx = (y * width as i32 + x) as usize;
                if idx < filtered.len() && !values.is_empty() {
                    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    filtered[idx] = values[values.len() / 2]; // Медиана
                }
            }
        }

        filtered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deduplication_spatial() {
        let mut system = DeduplicationSystem::new(2.0, 100);

        let meas1 = TemperatureMeasurement {
            x_mm: 0.0,
            y_mm: 0.0,
            angle_dms: "0:0:0".to_string(),
            temp_min: 25.0,
            temp_max: 30.0,
            timestamp: SystemTime::now(),
            pixel_x: 100,
            pixel_y: 100,
        };

        let meas2 = TemperatureMeasurement {
            x_mm: 0.0,
            y_mm: 0.0,
            angle_dms: "0:0:0".to_string(),
            temp_min: 25.0,
            temp_max: 30.0,
            timestamp: SystemTime::now(),
            pixel_x: 101, // В пределах радиуса
            pixel_y: 100,
        };

        assert!(system.check_and_add(meas1.clone()));
        assert!(!system.check_and_add(meas2)); // Должен быть дубликатом
    }
}
