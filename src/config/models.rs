// Типы конфигурации

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Основная конфигурация приложения
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub app: AppSettings,
    pub io: IOSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub version: u32,
    pub data_dir: PathBuf,
    pub logs_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IOSettings {
    pub controller_poll_ms: u32,
    pub camera_poll_ms: u32,
    pub serial_read_timeout_ms: u32,
}

/// Конфигурация сканирования
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    pub scan: ScanSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSettings {
    pub radius_mm: f64,
    pub pitch_mm: f64,
    pub angle: String, // Формат d:m:s или градусы
    pub dwell_ms: u32,
    pub order: String, // "concentric" | "spiral" | "grid"
    pub fast_image_mode: bool,
}

/// Конфигурация калибровки камеры
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraCalibrationConfig {
    pub camera: CameraCalibrationSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraCalibrationSettings {
    pub model: String,
    pub transform: Vec<f64>, // Матрица 3x3 сериализованная как 9 элементов
    pub roi: String, // Формат "x0,y0,w,h"
    pub cell_size_px: u32,
    pub version: u32,
}

/// Конфигурация механической калибровки
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerCalibrationConfig {
    pub mechanics: MechanicsSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MechanicsSettings {
    pub steps_per_rev: u32,
    pub lead_screw_pitch_mm: f64,
    pub microsteps: u32,
    pub pulses_per_mm: f64,
    pub encoder_cpr: u32,
    pub pulses_per_degree: f64,
    pub homing_speed_mm_s: f64,
    pub homing_offsets_mm: (f64, f64), // (x, y)
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app: AppSettings {
                version: 1,
                data_dir: PathBuf::from("./data"),
                logs_dir: PathBuf::from("./logs"),
            },
            io: IOSettings {
                controller_poll_ms: 5,
                camera_poll_ms: 10,
                serial_read_timeout_ms: 50,
            },
        }
    }
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            scan: ScanSettings {
                radius_mm: 50.0,
                pitch_mm: 0.25,
                angle: "0:1:0".to_string(),
                dwell_ms: 250,
                order: "concentric".to_string(),
                fast_image_mode: true,
            },
        }
    }
}

impl Default for ControllerCalibrationConfig {
    fn default() -> Self {
        Self {
            mechanics: MechanicsSettings {
                steps_per_rev: 200,
                lead_screw_pitch_mm: 8.0,
                microsteps: 16,
                pulses_per_mm: 3200.0,
                encoder_cpr: 1024,
                pulses_per_degree: 284.4,
                homing_speed_mm_s: 5.0,
                homing_offsets_mm: (0.0, 0.0),
            },
        }
    }
}

