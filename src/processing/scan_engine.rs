use std::sync::Arc;
use std::time::Duration;

use tokio::time::sleep;

use crate::hardware::camera::{CameraError, CameraMode, CameraService};
use crate::hardware::serial_communication::{
    ControllerCommand, ControllerResponse, SerialController, SerialError,
};

#[derive(Debug, Clone)]
pub struct ScanMeasurement {
    pub index: usize,
    pub position: (f32, f32),
    pub frame: crate::hardware::camera::CameraFrame,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ScanError {
    #[error("Controller error: {0}")]
    Controller(SerialError),
    #[error("Camera error: {0}")]
    Camera(CameraError),
    #[error("Unexpected controller response: {0}")]
    UnexpectedResponse(String),
}

/// Execute a scan by iterating through target positions, issuing controller moves, and capturing frames.
pub async fn execute_scan(
    controller: Arc<SerialController>,
    camera: Arc<CameraService>,
    positions: Vec<(f32, f32)>,
    mode: CameraMode,
    dwell: Duration,
    move_speed_mm_s: f32,
    calibrate_before_scan: bool,
    capture_timeout_ms: u16,
) -> Result<Vec<ScanMeasurement>, ScanError> {
    if positions.is_empty() {
        return Ok(Vec::new());
    }

    if calibrate_before_scan {
        let response = controller
            .send_command(ControllerCommand::Home)
            .await
            .map_err(ScanError::Controller)?;
        match response {
            ControllerResponse::HomeComplete => {}
            ControllerResponse::Error(message) => {
                return Err(ScanError::UnexpectedResponse(message))
            }
            other => {
                return Err(ScanError::UnexpectedResponse(format!(
                    "HOME returned {:?}",
                    other
                )))
            }
        }
    }

    let mut measurements = Vec::with_capacity(positions.len());

    for (index, &(x_mm, y_mm)) in positions.iter().enumerate() {
        let response = controller
            .send_command(ControllerCommand::Move {
                x_mm,
                speed_mm_s: move_speed_mm_s,
            })
            .await
            .map_err(ScanError::Controller)?;

        match response {
            ControllerResponse::MoveComplete => {}
            ControllerResponse::Error(message) => {
                return Err(ScanError::UnexpectedResponse(message))
            }
            other => {
                return Err(ScanError::UnexpectedResponse(format!(
                    "MOVE returned {:?}",
                    other
                )))
            }
        }

        if !dwell.is_zero() {
            sleep(dwell).await;
        }

        let frame = camera
            .capture_frame(mode.clone(), capture_timeout_ms)
            .await
            .map_err(ScanError::Camera)?;

        measurements.push(ScanMeasurement {
            index,
            position: (x_mm, y_mm),
            frame,
        });
    }

    Ok(measurements)
}
