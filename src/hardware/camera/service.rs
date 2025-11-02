use std::sync::{Arc, Mutex};

use tokio::task;

use crate::processing::analysis::{FastImageAnalyzer, PreciseImageAnalyzer};

use super::{
    backend::CameraFrameMetadata, BackendKind, CameraError, FrameDimensions, OptrisCamera,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraModeKind {
    Fast,
    Precise,
}

#[derive(Debug, Clone)]
pub struct FastModeSettings {
    pub roi: Option<(u32, u32, u32, u32)>,
    pub downscale_factor: u32,
}

impl Default for FastModeSettings {
    fn default() -> Self {
        Self {
            roi: None,
            downscale_factor: 4,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PreciseModeSettings {
    pub median_kernel_size: u32,
}

impl Default for PreciseModeSettings {
    fn default() -> Self {
        Self {
            median_kernel_size: 3,
        }
    }
}

#[derive(Debug, Clone)]
pub enum CameraMode {
    Fast(FastModeSettings),
    Precise(PreciseModeSettings),
}

impl CameraMode {
    pub fn kind(&self) -> CameraModeKind {
        match self {
            CameraMode::Fast(_) => CameraModeKind::Fast,
            CameraMode::Precise(_) => CameraModeKind::Precise,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CameraFrame {
    pub temperatures: Vec<f32>,
    pub width: u32,
    pub height: u32,
    pub min_temp: f32,
    pub max_temp: f32,
    pub metadata: CameraFrameMetadata,
    pub mode: CameraModeKind,
}

pub struct CameraService {
    camera: Arc<Mutex<OptrisCamera>>,
    dimensions: FrameDimensions,
    fast_defaults: FastModeSettings,
    precise_defaults: PreciseModeSettings,
}

impl CameraService {
    pub async fn connect(
        index: u16,
        backend_kind: BackendKind,
        instance_name: Option<String>,
    ) -> Result<Arc<Self>, CameraError> {
        let backend_clone = backend_kind;
        let instance = instance_name;
        let handle = task::spawn_blocking(
            move || -> Result<(OptrisCamera, FrameDimensions), CameraError> {
                let mut camera = OptrisCamera::with_backend(index, backend_clone);
                if let Err(err) = match instance.as_deref().filter(|name| !name.is_empty()) {
                    Some(name) => camera.init_named(name),
                    None => camera.init(),
                } {
                    return Err(CameraError::InitFailed(err.to_string()));
                }

                let dimensions = camera.frame_dimensions();
                Ok((camera, dimensions))
            },
        );

        let (camera, dimensions) = handle
            .await
            .map_err(|err| CameraError::InitFailed(format!("Join error: {err}")))??;

        let service = CameraService {
            camera: Arc::new(Mutex::new(camera)),
            dimensions,
            fast_defaults: FastModeSettings::default(),
            precise_defaults: PreciseModeSettings::default(),
        };

        Ok(Arc::new(service))
    }

    pub fn dimensions(&self) -> FrameDimensions {
        self.dimensions
    }

    pub fn default_fast_settings(&self) -> FastModeSettings {
        self.fast_defaults.clone()
    }

    pub fn default_precise_settings(&self) -> PreciseModeSettings {
        self.precise_defaults.clone()
    }

    pub async fn capture_frame(
        &self,
        mode: CameraMode,
        timeout_ms: u16,
    ) -> Result<CameraFrame, CameraError> {
        let camera = self.camera.clone();
        let dimensions = self.dimensions;

        let mut frame = task::spawn_blocking(move || {
            let mut camera = camera
                .lock()
                .map_err(|_| CameraError::CaptureFailed("Camera access poisoned".into()))?;

            let (raw_pixels, metadata) = camera
                .get_raw_frame(timeout_ms)
                .map_err(|err| CameraError::CaptureFailed(err.to_string()))?;
            let temps = camera.convert_to_temperature_celsius(&raw_pixels);
            let dims = camera.frame_dimensions();
            drop(camera);

            let width = dims.width.max(0) as u32;
            let height = dims.height.max(0) as u32;

            let (min_temp, max_temp) = match &mode {
                CameraMode::Fast(settings) => {
                    let analyzer =
                        FastImageAnalyzer::with_downscale_factor(settings.downscale_factor);
                    analyzer.analyze_fast(&temps, width, height, settings.roi)
                }
                CameraMode::Precise(settings) => {
                    let analyzer =
                        PreciseImageAnalyzer::with_kernel_size(settings.median_kernel_size);
                    analyzer.analyze_precise(&temps, width, height)
                }
            };

            Ok::<_, CameraError>(CameraFrame {
                temperatures: temps,
                width,
                height,
                min_temp,
                max_temp,
                metadata,
                mode: mode.kind(),
            })
        })
        .await
        .map_err(|err| CameraError::CaptureFailed(format!("Join error: {err}")))??;

        if frame.width == 0 || frame.height == 0 {
            frame.width = dimensions.width.max(0) as u32;
            frame.height = dimensions.height.max(0) as u32;
        }

        Ok(frame)
    }
}
