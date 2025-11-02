use std::time::Duration;

use thiserror::Error;

/// Dimensions of the thermal frame reported by the backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameDimensions {
    pub width: i32,
    pub height: i32,
    pub depth: i32,
}

impl FrameDimensions {
    pub fn zero() -> Self {
        Self {
            width: 0,
            height: 0,
            depth: 0,
        }
    }

    pub fn pixel_count(&self) -> usize {
        (self.width.max(0) as usize) * (self.height.max(0) as usize)
    }
}

/// Simplified metadata accompanying a captured thermal frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CameraFrameMetadata {
    pub frame_counter: u32,
    pub hardware_counter: u32,
    pub timestamp_us: i64,
    pub flag_state: u8,
}

/// Raw frame returned by a backend (prior to temperature conversion).
#[derive(Debug, Clone)]
pub struct RawFrame {
    pub pixels: Vec<u16>,
    pub metadata: CameraFrameMetadata,
}

/// Conversion parameters for translating ADU values into ?C.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TemperatureConversion {
    pub scale: f32,
    pub offset: f32,
}

impl Default for TemperatureConversion {
    fn default() -> Self {
        Self {
            scale: 0.1,
            offset: -100.0,
        }
    }
}

/// Common camera backend errors.
#[derive(Debug, Error)]
pub enum CameraError {
    #[error("Camera backend not available on this platform")]
    Unsupported,

    #[error("Camera not initialized")]
    NotInitialized,

    #[error("Failed to initialize camera: {0}")]
    InitFailed(String),

    #[error("Failed to capture frame: {0}")]
    CaptureFailed(String),

    #[error("Invalid frame buffer size")]
    InvalidFrame,
}

/// Trait implemented by concrete camera backends (real SDK or mock).
pub trait CameraBackend: Send {
    /// Prepare the backend for streaming.
    fn prepare(&mut self, instance_name: Option<&str>) -> Result<FrameDimensions, CameraError>;

    /// Acquire a single raw frame with the specified timeout.
    fn grab_raw_frame(&mut self, timeout: Duration) -> Result<RawFrame, CameraError>;

    /// Temperature conversion parameters. Backends can override to provide calibrated factors.
    fn temperature_conversion(&self) -> TemperatureConversion {
        TemperatureConversion::default()
    }

    /// Report the current chip temperature, if available.
    fn chip_temperature(&self) -> Option<f32> {
        None
    }

    /// Report the current flag temperature, if available.
    fn flag_temperature(&self) -> Option<f32> {
        None
    }

    /// Report the camera serial number, if available.
    fn serial_number(&self) -> Option<u32> {
        None
    }

    /// Shutdown hook giving the backend an opportunity to release resources.
    fn shutdown(&mut self) {}

    /// Human-readable name for diagnostics.
    fn backend_name(&self) -> &'static str;
}

/// Calculate temperature values using backend-provided conversion parameters.
pub fn convert_raw_to_temperature(raw: &[u16], conversion: TemperatureConversion) -> Vec<f32> {
    raw.iter()
        .map(|&adu| (adu as f32) * conversion.scale + conversion.offset)
        .collect()
}
