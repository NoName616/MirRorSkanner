// Структурированное логирование с ротацией файлов

use chrono::Local;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Менеджер логирования с поддержкой ротации файлов
pub struct LogManager {
    controller_log: Arc<Mutex<LogFile>>,
    camera_log: Arc<Mutex<LogFile>>,
    app_log: Arc<Mutex<LogFile>>,
    logs_dir: PathBuf,
    max_file_size: u64, // в байтах
}

/// Представление одного лог-файла
struct LogFile {
    path: PathBuf,
    current_size: u64,
    max_size: u64,
}

impl LogManager {
    /// Создает новый менеджер логирования
    pub fn new(logs_dir: impl AsRef<Path>) -> io::Result<Self> {
        let logs_dir = logs_dir.as_ref().to_path_buf();

        // Создаем директорию логов, если не существует
        std::fs::create_dir_all(&logs_dir)?;

        let max_file_size = 100 * 1024 * 1024; // 100 MB

        Ok(Self {
            controller_log: Arc::new(Mutex::new(LogFile::new(
                logs_dir.join("controller.log"),
                max_file_size,
            )?)),
            camera_log: Arc::new(Mutex::new(LogFile::new(
                logs_dir.join("camera.log"),
                max_file_size,
            )?)),
            app_log: Arc::new(Mutex::new(LogFile::new(
                logs_dir.join("app.log"),
                max_file_size,
            )?)),
            logs_dir,
            max_file_size,
        })
    }

    /// Записывает лог в файл контроллера
    pub fn log_controller(&self, level: LogLevel, message: &str) -> io::Result<()> {
        self.log_to_file(&self.controller_log, level, message)
    }

    /// Записывает лог в файл камеры
    pub fn log_camera(&self, level: LogLevel, message: &str) -> io::Result<()> {
        self.log_to_file(&self.camera_log, level, message)
    }

    /// Записывает лог в файл приложения
    pub fn log_app(&self, level: LogLevel, message: &str) -> io::Result<()> {
        self.log_to_file(&self.app_log, level, message)
    }

    /// Записывает лог в указанный файл с проверкой ротации
    fn log_to_file(
        &self,
        log_file: &Arc<Mutex<LogFile>>,
        level: LogLevel,
        message: &str,
    ) -> io::Result<()> {
        let mut log_file = log_file.lock().unwrap();

        // Проверяем, нужно ли делать ротацию
        if log_file.current_size >= log_file.max_size {
            log_file.rotate()?;
        }

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_entry = format!("[{}] [{}] {}\n", timestamp, level, message);
        let log_bytes = log_entry.as_bytes();

        // Открываем файл для записи
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file.path)?;

        file.write_all(log_bytes)?;
        file.flush()?;

        log_file.current_size += log_bytes.len() as u64;

        Ok(())
    }

    /// Очищает старые лог-файлы (старше указанного количества дней)
    pub fn clean_old_logs(&self, days: u32) -> io::Result<()> {
        let cutoff_time = chrono::Local::now() - chrono::Duration::days(days as i64);

        // Ищем все .log и .log.gz файлы в директории логов
        if let Ok(entries) = std::fs::read_dir(&self.logs_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "log" || ext == "gz" {
                            if let Ok(metadata) = path.metadata() {
                                if let Ok(modified) = metadata.modified() {
                                    let modified_time =
                                        chrono::DateTime::<chrono::Local>::from(modified);
                                    if modified_time < cutoff_time {
                                        std::fs::remove_file(&path)?;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl LogFile {
    fn new(path: PathBuf, max_size: u64) -> io::Result<Self> {
        let current_size = if path.exists() {
            path.metadata()?.len()
        } else {
            0
        };

        Ok(Self {
            path,
            current_size,
            max_size,
        })
    }

    /// Выполняет ротацию лог-файла (архивирует и создает новый)
    fn rotate(&mut self) -> io::Result<()> {
        if !self.path.exists() {
            return Ok(());
        }

        // Создаем имя архивного файла с timestamp
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let archive_name = format!(
            "{}.{}",
            self.path.file_stem().unwrap().to_string_lossy(),
            timestamp
        );
        let archive_path = self
            .path
            .parent()
            .unwrap()
            .join(format!("{}.log.gz", archive_name));

        // Читаем содержимое файла
        let contents = std::fs::read(&self.path)?;

        // Сжимаем содержимое (простая реализация - можно использовать flate2)
        // Для упрощения просто архивируем без сжатия
        // В реальной реализации можно использовать flate2 или другой компрессор
        std::fs::write(&archive_path, contents)?;

        // Очищаем исходный файл
        std::fs::remove_file(&self.path)?;
        File::create(&self.path)?;

        self.current_size = 0;

        Ok(())
    }
}

/// Уровни логирования
#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warning => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

/// Глобальный экземпляр менеджера логирования
static GLOBAL_LOG_MANAGER: std::sync::OnceLock<Arc<LogManager>> = std::sync::OnceLock::new();

/// Инициализирует глобальный менеджер логирования
pub fn init_log_manager(logs_dir: impl AsRef<Path>) -> io::Result<()> {
    let manager = Arc::new(LogManager::new(logs_dir)?);
    GLOBAL_LOG_MANAGER.set(manager).map_err(|_| {
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Log manager already initialized",
        )
    })?;
    Ok(())
}

/// Получает глобальный менеджер логирования
pub fn get_log_manager() -> Option<Arc<LogManager>> {
    GLOBAL_LOG_MANAGER.get().cloned()
}

/// Вспомогательные макросы для логирования
#[macro_export]
macro_rules! log_controller {
    ($level:expr, $($arg:tt)*) => {
        if let Some(manager) = $crate::utils::logging::get_log_manager() {
            let _ = manager.log_controller($level, &format!($($arg)*));
        }
    };
}

#[macro_export]
macro_rules! log_camera {
    ($level:expr, $($arg:tt)*) => {
        if let Some(manager) = $crate::utils::logging::get_log_manager() {
            let _ = manager.log_camera($level, &format!($($arg)*));
        }
    };
}

#[macro_export]
macro_rules! log_app {
    ($level:expr, $($arg:tt)*) => {
        if let Some(manager) = $crate::utils::logging::get_log_manager() {
            let _ = manager.log_app($level, &format!($($arg)*));
        }
    };
}
