#![cfg(target_os = "windows")]

use std::time::Duration;

use super::backend::{
    CameraBackend, CameraError, CameraFrameMetadata, FrameDimensions, RawFrame,
    TemperatureConversion,
};
use crate::hardware::ffi;

pub struct OptrisSdkBackend {
    index: u16,
    dims: Option<FrameDimensions>,
    is_running: bool,
}

impl OptrisSdkBackend {
    pub fn new(index: u16) -> Self {
        Self {
            index,
            dims: None,
            is_running: false,
        }
    }

    fn ensure_dims(&self) -> Result<FrameDimensions, CameraError> {
        self.dims.ok_or(CameraError::NotInitialized)
    }
}

impl CameraBackend for OptrisSdkBackend {
    fn prepare(&mut self, instance_name: Option<&str>) -> Result<FrameDimensions, CameraError> {
        let init_result = if let Some(name) = instance_name {
            ffi::init_named_imager_ipc(self.index, name)
        } else {
            ffi::init_imager_ipc(self.index)
        };

        init_result.map_err(|err| CameraError::InitFailed(format!("HRESULT {err}")))?;

        let (width, height, depth) = ffi::get_frame_config(self.index)
            .map_err(|err| CameraError::InitFailed(format!("frame config HRESULT {err}")))?;

        ffi::run_imager_ipc(self.index)
            .map_err(|err| CameraError::InitFailed(format!("run HRESULT {err}")))?;
        ffi::start_imager_ipc(self.index)
            .map_err(|err| CameraError::InitFailed(format!("start HRESULT {err}")))?;

        let dims = FrameDimensions {
            width,
            height,
            depth,
        };
        self.dims = Some(dims);
        self.is_running = true;

        Ok(dims)
    }

    fn grab_raw_frame(&mut self, timeout: Duration) -> Result<RawFrame, CameraError> {
        let dims = self.ensure_dims()?;
        let mut buffer = vec![0u16; dims.pixel_count()];
        let mut metadata = ffi::FrameMetadata {
            size: 0,
            counter: 0,
            counter_hw: 0,
            timestamp: 0,
            timestamp_media: 0,
            flag_state: ffi::TFlagState::FsError,
            temp_chip: 0.0,
            temp_flag: 0.0,
            temp_box: 0.0,
            pif_in: [0, 0],
        };

        // reinterpret buffer as bytes
        let buffer_bytes = unsafe {
            std::slice::from_raw_parts_mut(
                buffer.as_mut_ptr() as *mut u8,
                buffer.len() * std::mem::size_of::<u16>(),
            )
        };

        ffi::get_frame(
            self.index,
            timeout.as_millis().min(u16::MAX as u128) as u16,
            buffer_bytes,
            &mut metadata,
        )
        .map_err(|err| CameraError::CaptureFailed(format!("HRESULT {err}")))?;

        let meta = CameraFrameMetadata {
            frame_counter: metadata.counter,
            hardware_counter: metadata.counter_hw,
            timestamp_us: metadata.timestamp,
            flag_state: metadata.flag_state as u8,
        };

        Ok(RawFrame {
            pixels: buffer,
            metadata: meta,
        })
    }

    fn temperature_conversion(&self) -> TemperatureConversion {
        TemperatureConversion::default()
    }

    fn chip_temperature(&self) -> Option<f32> {
        if self.is_running {
            Some(ffi::get_temp_chip(self.index))
        } else {
            None
        }
    }

    fn flag_temperature(&self) -> Option<f32> {
        if self.is_running {
            Some(ffi::get_temp_flag(self.index))
        } else {
            None
        }
    }

    fn serial_number(&self) -> Option<u32> {
        if self.is_running {
            Some(ffi::get_serial_number(self.index))
        } else {
            None
        }
    }

    fn shutdown(&mut self) {
        if self.is_running {
            let _ = ffi::release_imager_ipc(self.index);
            self.is_running = false;
        }
    }

    fn backend_name(&self) -> &'static str {
        "Optris Connect SDK"
    }
}
