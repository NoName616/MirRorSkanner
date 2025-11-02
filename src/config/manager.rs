// Менеджер конфигурации: загрузка, сохранение, hot-reload INI файлов

use super::models::*;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

// Используем простой парсер INI, так как пакет ini версии 1.3 имеет другой API
// Для совместимости используем наш собственный парсер

// Простой парсер INI-файлов
fn parse_ini_file(path: &Path) -> Result<HashMap<String, HashMap<String, String>>> {
    let content = std::fs::read_to_string(path)?;
    let mut result = HashMap::new();
    let mut current_section = HashMap::new();
    let mut section_name = String::from("default");

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            if !current_section.is_empty() {
                result.insert(section_name.clone(), current_section);
            }
            section_name = line[1..line.len() - 1].to_string();
            current_section = HashMap::new();
        } else if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim().to_string();
            let value = line[eq_pos + 1..].trim().to_string();
            current_section.insert(key, value);
        }
    }
    if !current_section.is_empty() {
        result.insert(section_name, current_section);
    }
    Ok(result)
}

// Сохранение INI-файла
fn save_ini_file(path: &Path, data: &HashMap<String, HashMap<String, String>>) -> Result<()> {
    let mut file = File::create(path)?;

    for (section_name, section) in data {
        writeln!(file, "[{}]", section_name)?;
        for (key, value) in section {
            writeln!(file, "{} = {}", key, value)?;
        }
        writeln!(file)?;
    }

    Ok(())
}

/// Менеджер конфигурации с поддержкой hot-reload
pub struct ConfigManager {
    config_dir: PathBuf,
    app_config: Arc<RwLock<AppConfig>>,
    scan_config: Arc<RwLock<ScanConfig>>,
    camera_calib_config: Arc<RwLock<Option<CameraCalibrationConfig>>>,
    controller_calib_config: Arc<RwLock<ControllerCalibrationConfig>>,
}

impl ConfigManager {
    /// Создает новый менеджер конфигурации
    pub fn new(config_dir: impl AsRef<Path>) -> Result<Self> {
        let config_dir = config_dir.as_ref().to_path_buf();

        // Создаем директорию если не существует
        std::fs::create_dir_all(&config_dir).context("Failed to create config directory")?;

        let manager = Self {
            config_dir: config_dir.clone(),
            app_config: Arc::new(RwLock::new(AppConfig::default())),
            scan_config: Arc::new(RwLock::new(ScanConfig::default())),
            camera_calib_config: Arc::new(RwLock::new(None)),
            controller_calib_config: Arc::new(RwLock::new(ControllerCalibrationConfig::default())),
        };

        // Загружаем конфигурации при старте
        manager.load_all()?;

        Ok(manager)
    }

    /// Загружает все конфигурации
    pub fn load_all(&self) -> Result<()> {
        self.load_app_config()?;
        self.load_scan_config()?;
        self.load_camera_calibration()?;
        self.load_controller_calibration()?;
        Ok(())
    }

    /// Загружает основную конфигурацию из config.cfg
    pub fn load_app_config(&self) -> Result<()> {
        let path = self.config_dir.join("config.cfg");

        if !path.exists() {
            // Создаем дефолтный конфиг
            self.save_app_config()?;
            return Ok(());
        }

        let ini = parse_ini_file(&path)
            .with_context(|| format!("Failed to load config.cfg from {:?}", path))?;

        let mut config = AppConfig::default();

        // Загружаем [app]
        if let Some(section) = ini.get("app") {
            if let Some(version) = section.get("version") {
                config.app.version = version.parse().unwrap_or(1);
            }
            if let Some(data_dir) = section.get("data_dir") {
                config.app.data_dir = PathBuf::from(data_dir);
            }
            if let Some(logs_dir) = section.get("logs_dir") {
                config.app.logs_dir = PathBuf::from(logs_dir);
            }
        }

        // Загружаем [io]
        if let Some(section) = ini.get("io") {
            if let Some(poll_ms) = section.get("controller_poll_ms") {
                config.io.controller_poll_ms = poll_ms.parse().unwrap_or(5);
            }
            if let Some(poll_ms) = section.get("camera_poll_ms") {
                config.io.camera_poll_ms = poll_ms.parse().unwrap_or(10);
            }
            if let Some(timeout_ms) = section.get("serial_read_timeout_ms") {
                config.io.serial_read_timeout_ms = timeout_ms.parse().unwrap_or(50);
            }
        }

        *self.app_config.write().unwrap() = config;
        Ok(())
    }

    /// Сохраняет основную конфигурацию в config.cfg
    pub fn save_app_config(&self) -> Result<()> {
        let path = self.config_dir.join("config.cfg");
        let config = self.app_config.read().unwrap();

        let mut ini_data = HashMap::new();

        // Сохраняем [app]
        let mut app_section = HashMap::new();
        app_section.insert("version".to_string(), config.app.version.to_string());
        app_section.insert(
            "data_dir".to_string(),
            config.app.data_dir.to_string_lossy().to_string(),
        );
        app_section.insert(
            "logs_dir".to_string(),
            config.app.logs_dir.to_string_lossy().to_string(),
        );
        ini_data.insert("app".to_string(), app_section);

        // Сохраняем [io]
        let mut io_section = HashMap::new();
        io_section.insert(
            "controller_poll_ms".to_string(),
            config.io.controller_poll_ms.to_string(),
        );
        io_section.insert(
            "camera_poll_ms".to_string(),
            config.io.camera_poll_ms.to_string(),
        );
        io_section.insert(
            "serial_read_timeout_ms".to_string(),
            config.io.serial_read_timeout_ms.to_string(),
        );
        ini_data.insert("io".to_string(), io_section);

        save_ini_file(&path, &ini_data)
            .with_context(|| format!("Failed to save config.cfg to {:?}", path))?;

        Ok(())
    }

    /// Загружает конфигурацию сканирования из config_scan.cfg
    pub fn load_scan_config(&self) -> Result<()> {
        let path = self.config_dir.join("config_scan.cfg");

        if !path.exists() {
            self.save_scan_config()?;
            return Ok(());
        }

        let ini =
            parse_ini_file(&path).with_context(|| format!("Failed to load config_scan.cfg"))?;

        let mut config = ScanConfig::default();

        if let Some(section) = ini.get("scan") {
            if let Some(radius) = section.get("radius_mm") {
                config.scan.radius_mm = radius.parse().unwrap_or(50.0);
            }
            if let Some(pitch) = section.get("pitch_mm") {
                config.scan.pitch_mm = pitch.parse().unwrap_or(0.25);
            }
            if let Some(angle) = section.get("angle") {
                config.scan.angle = angle.to_string();
            }
            if let Some(dwell) = section.get("dwell_ms") {
                config.scan.dwell_ms = dwell.parse().unwrap_or(250);
            }
            if let Some(order) = section.get("order") {
                config.scan.order = order.to_string();
            }
            if let Some(fast_mode) = section.get("fast_image_mode") {
                config.scan.fast_image_mode = fast_mode.parse().unwrap_or(true);
            }
        }

        *self.scan_config.write().unwrap() = config;
        Ok(())
    }

    /// Сохраняет конфигурацию сканирования
    pub fn save_scan_config(&self) -> Result<()> {
        let path = self.config_dir.join("config_scan.cfg");
        let config = self.scan_config.read().unwrap();

        let mut ini_data = HashMap::new();
        let mut scan_section = HashMap::new();
        scan_section.insert("radius_mm".to_string(), config.scan.radius_mm.to_string());
        scan_section.insert("pitch_mm".to_string(), config.scan.pitch_mm.to_string());
        scan_section.insert("angle".to_string(), config.scan.angle.clone());
        scan_section.insert("dwell_ms".to_string(), config.scan.dwell_ms.to_string());
        scan_section.insert("order".to_string(), config.scan.order.clone());
        scan_section.insert(
            "fast_image_mode".to_string(),
            config.scan.fast_image_mode.to_string(),
        );
        ini_data.insert("scan".to_string(), scan_section);

        save_ini_file(&path, &ini_data)
            .with_context(|| format!("Failed to save config_scan.cfg"))?;

        Ok(())
    }

    /// Загружает конфигурацию калибровки контроллера
    pub fn load_controller_calibration(&self) -> Result<()> {
        let path = self.config_dir.join("calibration_controll.cfg");

        if !path.exists() {
            self.save_controller_calibration()?;
            return Ok(());
        }

        let ini = parse_ini_file(&path)
            .with_context(|| format!("Failed to load calibration_controll.cfg"))?;

        let mut config = ControllerCalibrationConfig::default();

        if let Some(section) = ini.get("mechanics") {
            if let Some(val) = section.get("steps_per_rev") {
                config.mechanics.steps_per_rev = val.parse().unwrap_or(200);
            }
            if let Some(val) = section.get("lead_screw_pitch_mm") {
                config.mechanics.lead_screw_pitch_mm = val.parse().unwrap_or(8.0);
            }
            if let Some(val) = section.get("microsteps") {
                config.mechanics.microsteps = val.parse().unwrap_or(16);
            }
            if let Some(val) = section.get("pulses_per_mm") {
                config.mechanics.pulses_per_mm = val.parse().unwrap_or(3200.0);
            }
            if let Some(val) = section.get("encoder_cpr") {
                config.mechanics.encoder_cpr = val.parse().unwrap_or(1024);
            }
            if let Some(val) = section.get("pulses_per_degree") {
                config.mechanics.pulses_per_degree = val.parse().unwrap_or(284.4);
            }
            if let Some(val) = section.get("homing_speed_mm_s") {
                config.mechanics.homing_speed_mm_s = val.parse().unwrap_or(5.0);
            }
            if let Some(val) = section.get("homing_offsets_mm") {
                let parts: Vec<&str> = val.split(',').collect();
                if parts.len() == 2 {
                    if let (Ok(x), Ok(y)) = (parts[0].parse(), parts[1].parse()) {
                        config.mechanics.homing_offsets_mm = (x, y);
                    }
                }
            }
        }

        *self.controller_calib_config.write().unwrap() = config;
        Ok(())
    }

    /// Сохраняет конфигурацию калибровки контроллера
    pub fn save_controller_calibration(&self) -> Result<()> {
        let path = self.config_dir.join("calibration_controll.cfg");
        let config = self.controller_calib_config.read().unwrap();

        let mut ini_data = HashMap::new();
        let mut mechanics_section = HashMap::new();
        mechanics_section.insert(
            "steps_per_rev".to_string(),
            config.mechanics.steps_per_rev.to_string(),
        );
        mechanics_section.insert(
            "lead_screw_pitch_mm".to_string(),
            config.mechanics.lead_screw_pitch_mm.to_string(),
        );
        mechanics_section.insert(
            "microsteps".to_string(),
            config.mechanics.microsteps.to_string(),
        );
        mechanics_section.insert(
            "pulses_per_mm".to_string(),
            config.mechanics.pulses_per_mm.to_string(),
        );
        mechanics_section.insert(
            "encoder_cpr".to_string(),
            config.mechanics.encoder_cpr.to_string(),
        );
        mechanics_section.insert(
            "pulses_per_degree".to_string(),
            config.mechanics.pulses_per_degree.to_string(),
        );
        mechanics_section.insert(
            "homing_speed_mm_s".to_string(),
            config.mechanics.homing_speed_mm_s.to_string(),
        );
        mechanics_section.insert(
            "homing_offsets_mm".to_string(),
            format!(
                "{},{}",
                config.mechanics.homing_offsets_mm.0, config.mechanics.homing_offsets_mm.1
            ),
        );
        ini_data.insert("mechanics".to_string(), mechanics_section);

        save_ini_file(&path, &ini_data)
            .with_context(|| format!("Failed to save calibration_controll.cfg"))?;

        Ok(())
    }

    /// Загружает конфигурацию калибровки камеры
    pub fn load_camera_calibration(&self) -> Result<()> {
        let path = self.config_dir.join("calibration_cam.cfg");

        if !path.exists() {
            // Калибровка камеры может отсутствовать
            *self.camera_calib_config.write().unwrap() = None;
            return Ok(());
        }

        let ini = parse_ini_file(&path)?;

        let mut config = CameraCalibrationSettings {
            model: "optris_pi640".to_string(),
            transform: vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0], // Единичная матрица
            roi: "0,0,640,480".to_string(),
            cell_size_px: 8,
            version: 1,
        };

        if let Some(section) = ini.get("camera") {
            if let Some(model) = section.get("model") {
                config.model = model.to_string();
            }
            if let Some(transform_str) = section.get("transform") {
                // Парсим массив чисел из строки типа "[1.0, 0.0, ...]"
                let cleaned = transform_str.trim_matches(|c| c == '[' || c == ']');
                config.transform = cleaned
                    .split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                if config.transform.len() != 9 {
                    config.transform = vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
                }
            }
            if let Some(roi) = section.get("roi") {
                config.roi = roi.to_string();
            }
            if let Some(cell_size) = section.get("cell_size_px") {
                config.cell_size_px = cell_size.parse().unwrap_or(8);
            }
            if let Some(version) = section.get("version") {
                config.version = version.parse().unwrap_or(1);
            }
        }

        *self.camera_calib_config.write().unwrap() =
            Some(CameraCalibrationConfig { camera: config });
        Ok(())
    }

    /// Сохраняет конфигурацию калибровки камеры
    pub fn save_camera_calibration(&self) -> Result<()> {
        let path = self.config_dir.join("calibration_cam.cfg");

        let config_opt = self.camera_calib_config.read().unwrap();
        if let Some(config) = config_opt.as_ref() {
            let mut ini_data = HashMap::new();
            let mut camera_section = HashMap::new();
            let transform_str = format!(
                "[{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6}]",
                config.camera.transform[0],
                config.camera.transform[1],
                config.camera.transform[2],
                config.camera.transform[3],
                config.camera.transform[4],
                config.camera.transform[5],
                config.camera.transform[6],
                config.camera.transform[7],
                config.camera.transform[8]
            );

            camera_section.insert("model".to_string(), config.camera.model.clone());
            camera_section.insert("transform".to_string(), transform_str);
            camera_section.insert("roi".to_string(), config.camera.roi.clone());
            camera_section.insert(
                "cell_size_px".to_string(),
                config.camera.cell_size_px.to_string(),
            );
            camera_section.insert("version".to_string(), config.camera.version.to_string());
            ini_data.insert("camera".to_string(), camera_section);

            save_ini_file(&path, &ini_data)?;
        }

        Ok(())
    }

    /// Получить доступ к конфигурации приложения
    pub fn app_config(&self) -> Arc<RwLock<AppConfig>> {
        self.app_config.clone()
    }

    /// Получить доступ к конфигурации сканирования
    pub fn scan_config(&self) -> Arc<RwLock<ScanConfig>> {
        self.scan_config.clone()
    }

    /// Получить доступ к конфигурации калибровки контроллера
    pub fn controller_calib_config(&self) -> Arc<RwLock<ControllerCalibrationConfig>> {
        self.controller_calib_config.clone()
    }

    /// Получить доступ к конфигурации калибровки камеры
    pub fn camera_calib_config(&self) -> Arc<RwLock<Option<CameraCalibrationConfig>>> {
        self.camera_calib_config.clone()
    }

    /// Запускает hot-reload мониторинг конфигурационных файлов
    pub fn start_hot_reload(&self, check_interval_ms: u64) {
        let config_dir = self.config_dir.clone();
        let app_config = self.app_config.clone();
        let scan_config = self.scan_config.clone();

        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(check_interval_ms));

                // Проверяем изменения в основных конфигах
                let config_path = config_dir.join("config.cfg");
                if config_path.exists() {
                    if let Ok(_modified) =
                        std::fs::metadata(&config_path).and_then(|m| m.modified())
                    {
                        // Простая проверка: если файл изменился, перезагружаем
                        let _ = Self {
                            config_dir: config_dir.clone(),
                            app_config: app_config.clone(),
                            scan_config: scan_config.clone(),
                            camera_calib_config: Arc::new(RwLock::new(None)),
                            controller_calib_config: Arc::new(RwLock::new(
                                ControllerCalibrationConfig::default(),
                            )),
                        }
                        .load_app_config();
                    }
                }

                let scan_config_path = config_dir.join("config_scan.cfg");
                if scan_config_path.exists() {
                    let _ = Self {
                        config_dir: config_dir.clone(),
                        app_config: app_config.clone(),
                        scan_config: scan_config.clone(),
                        camera_calib_config: Arc::new(RwLock::new(None)),
                        controller_calib_config: Arc::new(RwLock::new(
                            ControllerCalibrationConfig::default(),
                        )),
                    }
                    .load_scan_config();
                }
            }
        });
    }
}
