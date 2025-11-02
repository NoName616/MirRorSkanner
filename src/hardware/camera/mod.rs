mod backend;
mod mock;
mod service;

#[cfg(target_os = "windows")]
mod optris;

use std::time::Duration;

use anyhow::Result;

pub use backend::{
    convert_raw_to_temperature, CameraError, CameraFrameMetadata, FrameDimensions, RawFrame,
    TemperatureConversion,
};
pub use service::{
    CameraFrame, CameraMode, CameraModeKind, CameraService, FastModeSettings, PreciseModeSettings,
};

use backend::CameraBackend;

/// Runtime camera abstraction that selects an appropriate backend at startup.
pub struct OptrisCamera {
    index: u16,
    backend: Box<dyn CameraBackend>,
    dimensions: FrameDimensions,
    is_initialized: bool,
    conversion: TemperatureConversion,
    active_backend_name: &'static str,
}

impl OptrisCamera {
    /// Create a new camera instance targeting the provided index.
    pub fn new(index: u16) -> Self {
        let backend = create_backend(index);
        let backend_name = backend.backend_name();

        Self {
            index,
            backend,
            dimensions: FrameDimensions::zero(),
            is_initialized: false,
            conversion: TemperatureConversion::default(),
            active_backend_name: backend_name,
        }
    }

    /// Create a camera using an explicitly selected backend kind ("mock" currently).
    pub fn with_backend(index: u16, backend_kind: BackendKind) -> Self {
        let backend: Box<dyn CameraBackend> = match backend_kind {
            BackendKind::Mock => Box::new(mock::MockCameraBackend::new()),
            BackendKind::OptrisSdk => select_optris_backend(index),
        };

        let backend_name = backend.backend_name();

        Self {
            index,
            backend,
            dimensions: FrameDimensions::zero(),
            is_initialized: false,
            conversion: TemperatureConversion::default(),
            active_backend_name: backend_name,
        }
    }

    fn init_with_instance(&mut self, instance_name: Option<&str>) -> Result<()> {
        let dims = self
            .backend
            .prepare(instance_name)
            .map_err(anyhow::Error::new)?;
        self.dimensions = dims;
        self.is_initialized = true;
        self.conversion = self.backend.temperature_conversion();
        self.active_backend_name = self.backend.backend_name();
        Ok(())
    }

    /// Initialize the camera using default Connect SDK discovery.
    pub fn init(&mut self) -> Result<()> {
        self.init_with_instance(None)
    }

    /// Initialize the camera referencing a named PIX Connect instance (Windows only).
    pub fn init_named(&mut self, instance_name: &str) -> Result<()> {
        self.init_with_instance(Some(instance_name))
    }

    /// Obtain raw frame dimensions.
    pub fn get_frame_size(&self) -> (i32, i32, i32) {
        (
            self.dimensions.width,
            self.dimensions.height,
            self.dimensions.depth,
        )
    }

    /// Returns cached frame dimensions as a struct.
    pub fn frame_dimensions(&self) -> FrameDimensions {
        self.dimensions
    }

    /// Acquire a raw frame with the provided timeout (milliseconds).
    pub fn get_raw_frame(&mut self, timeout_ms: u16) -> Result<(Vec<u16>, CameraFrameMetadata)> {
        if !self.is_initialized {
            return Err(CameraError::NotInitialized.into());
        }

        let timeout = Duration::from_millis(timeout_ms as u64);
        let RawFrame { pixels, metadata } = self
            .backend
            .grab_raw_frame(timeout)
            .map_err(anyhow::Error::new)?;

        Ok((pixels, metadata))
    }

    /// Convert raw ADU values into ?C using backend conversion parameters.
    pub fn convert_to_temperature_celsius(&self, raw_data: &[u16]) -> Vec<f32> {
        convert_raw_to_temperature(raw_data, self.conversion)
    }

    /// Returns the active temperature conversion coefficients.
    pub fn temperature_conversion(&self) -> TemperatureConversion {
        self.conversion
    }

    /// Retrieve the camera chip temperature (if available).
    pub fn get_chip_temperature(&self) -> f32 {
        self.backend.chip_temperature().unwrap_or(f32::NAN)
    }

    /// Retrieve the flag temperature (if available).
    pub fn get_flag_temperature(&self) -> f32 {
        self.backend.flag_temperature().unwrap_or(f32::NAN)
    }

    /// Retrieve the serial number (if available).
    pub fn get_serial_number(&self) -> u32 {
        self.backend.serial_number().unwrap_or_default()
    }

    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    pub fn backend_name(&self) -> &'static str {
        self.active_backend_name
    }

    /// Release backend resources.
    pub fn release(&mut self) -> Result<()> {
        self.backend.shutdown();
        self.is_initialized = false;
        Ok(())
    }
}

impl Drop for OptrisCamera {
    fn drop(&mut self) {
        let _ = self.release();
    }
}

/// Supported backend kinds (extensible via configuration).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    OptrisSdk,
    Mock,
}

fn create_backend(index: u16) -> Box<dyn CameraBackend> {
    let backend_override = std::env::var("MIRORSKANER_CAMERA_BACKEND").ok();

    match backend_override.as_deref() {
        Some(name) if name.eq_ignore_ascii_case("mock") => Box::new(mock::MockCameraBackend::new()),
        Some(name) if name.eq_ignore_ascii_case("optris") => select_optris_backend(index),
        _ => select_optris_backend(index),
    }
}

fn select_optris_backend(index: u16) -> Box<dyn CameraBackend> {
    #[cfg(target_os = "windows")]
    {
        Box::new(optris::OptrisSdkBackend::new(index))
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = index; // suppress unused variable warning
        Box::new(mock::MockCameraBackend::new())
    }
}
