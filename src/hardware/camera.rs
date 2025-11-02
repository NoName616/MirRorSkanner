// Работа с камерой Optris PI 640 через Connect SDK

use crate::hardware::ffi::*;
use std::sync::{Arc, Mutex};
use anyhow::Result;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CameraError {
    #[error("Camera not initialized")]
    NotInitialized,
    #[error("Failed to initialize camera: {0}")]
    InitError(HRESULT),
    #[error("Failed to get frame: {0}")]
    FrameError(HRESULT),
    #[error("Camera index out of range")]
    InvalidIndex,
}

/// Обертка для работы с камерой Optris
pub struct OptrisCamera {
    index: u16,
    width: i32,
    height: i32,
    depth: i32,
    is_initialized: bool,
    is_running: bool,
}

impl OptrisCamera {
    /// Создает новую камеру с указанным индексом
    pub fn new(index: u16) -> Self {
        Self {
            index,
            width: 0,
            height: 0,
            depth: 0,
            is_initialized: false,
            is_running: false,
        }
    }

    /// Инициализирует камеру через Connect SDK
    pub fn init(&mut self) -> Result<()> {
        let result = init_imager_ipc(self.index);
        match result {
            Ok(_) => {
                // Получаем конфигурацию кадра
                match get_frame_config(self.index) {
                    Ok((w, h, d)) => {
                        self.width = w;
                        self.height = h;
                        self.depth = d;
                    }
                    Err(e) => {
                        return Err(CameraError::InitError(e).into());
                    }
                }
                
                // Запускаем IPC
                run_imager_ipc(self.index)
                    .map_err(|e| CameraError::InitError(e))?;
                
                // Стартуем IPC
                start_imager_ipc(self.index)
                    .map_err(|e| CameraError::InitError(e))?;
                
                self.is_initialized = true;
                self.is_running = true;
                Ok(())
            }
            Err(e) => {
                Err(CameraError::InitError(e).into())
            }
        }
    }

    /// Инициализирует именованную камеру
    pub fn init_named(&mut self, instance_name: &str) -> Result<()> {
        let result = init_named_imager_ipc(self.index, instance_name);
        match result {
            Ok(_) => {
                match get_frame_config(self.index) {
                    Ok((w, h, d)) => {
                        self.width = w;
                        self.height = h;
                        self.depth = d;
                    }
                    Err(e) => {
                        return Err(CameraError::InitError(e).into());
                    }
                }
                
                run_imager_ipc(self.index)
                    .map_err(|e| CameraError::InitError(e))?;
                
                start_imager_ipc(self.index)
                    .map_err(|e| CameraError::InitError(e))?;
                
                self.is_initialized = true;
                self.is_running = true;
                Ok(())
            }
            Err(e) => {
                Err(CameraError::InitError(e).into())
            }
        }
    }

    /// Получает размер кадра
    pub fn get_frame_size(&self) -> (i32, i32, i32) {
        (self.width, self.height, self.depth)
    }

    /// Получает сырой кадр от камеры
    pub fn get_raw_frame(&self, timeout_ms: u16) -> Result<(Vec<u16>, FrameMetadata)> {
        if !self.is_initialized {
            return Err(CameraError::NotInitialized.into());
        }

        // Камера Optris PI 640 возвращает данные в формате u16 (16-битные значения)
        let frame_size = (self.width * self.height) as usize;
        let mut buffer = vec![0u16; frame_size];
        let mut metadata = FrameMetadata {
            size: 0,
            counter: 0,
            counter_hw: 0,
            timestamp: 0,
            timestamp_media: 0,
            flag_state: TFlagState::FsError,
            temp_chip: 0.0,
            temp_flag: 0.0,
            temp_box: 0.0,
            pif_in: [0, 0],
        };

        // Конвертируем буфер в u8 для передачи в FFI
        let buffer_bytes = unsafe {
            std::slice::from_raw_parts_mut(
                buffer.as_mut_ptr() as *mut u8,
                buffer.len() * std::mem::size_of::<u16>()
            )
        };

        let result = get_frame(
            self.index,
            timeout_ms,
            buffer_bytes,
            &mut metadata,
        );

        match result {
            Ok(_) => Ok((buffer, metadata)),
            Err(e) => Err(CameraError::FrameError(e).into()),
        }
    }

    /// Конвертирует сырые данные камеры в температуры в градусах Цельсия
    /// Оптис PI 640 возвращает данные в ADU (Analog-to-Digital Units), которые нужно конвертировать
    /// Формула: Temp = (ADU / 10.0) - 100.0 (примерная формула, может потребоваться калибровка)
    pub fn convert_to_temperature_celsius(&self, raw_data: &[u16]) -> Vec<f32> {
        // Базовое преобразование ADU в температуру
        // Для Optris PI 640: температура обычно рассчитывается как:
        // T = (ADU / масштаб) + смещение
        // Точные параметры зависят от модели и калибровки камеры
        
        // Приблизительная формула (требует уточнения):
        // Для PI 640 обычно: масштаб = 10, смещение зависит от диапазона
        raw_data.iter().map(|&adu| {
            // Базовое преобразование (требует калибровки для точности)
            (adu as f32 / 10.0) - 100.0
        }).collect()
    }

    /// Получает температуру чипа камеры
    pub fn get_chip_temperature(&self) -> f32 {
        if !self.is_initialized {
            return 0.0;
        }
        get_temp_chip(self.index)
    }

    /// Получает температуру флага (flag)
    pub fn get_flag_temperature(&self) -> f32 {
        if !self.is_initialized {
            return 0.0;
        }
        get_temp_flag(self.index)
    }

    /// Получает серийный номер камеры
    pub fn get_serial_number(&self) -> u32 {
        if !self.is_initialized {
            return 0;
        }
        get_serial_number(self.index)
    }

    /// Проверяет, инициализирована ли камера
    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    /// Освобождает ресурсы камеры
    pub fn release(&mut self) -> Result<()> {
        if self.is_initialized {
            release_imager_ipc(self.index)
                .map_err(|e| CameraError::InitError(e))?;
            self.is_initialized = false;
            self.is_running = false;
        }
        Ok(())
    }
}

impl Drop for OptrisCamera {
    fn drop(&mut self) {
        if self.is_initialized {
            let _ = self.release();
        }
    }
}

/// Безопасная обертка для многопоточного доступа к камере
pub type SharedCamera = Arc<Mutex<OptrisCamera>>;

