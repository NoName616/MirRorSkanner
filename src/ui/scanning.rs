use iced::widget::{button, checkbox, column, pick_list, progress_bar, row, text, text_input};
use iced::{Command, Element};
use std::f32::consts::PI;
use std::sync::Arc;
use std::time::Duration;

use crate::hardware::camera::{CameraMode, CameraService};
use crate::hardware::serial_communication::SerialController;
use crate::processing::scan_engine::{execute_scan, ScanError, ScanMeasurement};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ScanType {
    Grid,
    ConcentricCircle,
    Spiral,
}

impl std::fmt::Display for ScanType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScanType::Grid => write!(f, "Grid"),
            ScanType::ConcentricCircle => write!(f, "Concentric Circle"),
            ScanType::Spiral => write!(f, "Spiral"),
        }
    }
}

pub struct State {
    is_scanning: bool,
    scan_progress: f32,
    start_x: f32,
    start_y: f32,
    end_x: f32,
    end_y: f32,
    step_size: f32,
    scan_type: ScanType,
    radius: f32,
    angular_step: f32, // in degrees
    center_x: f32,
    center_y: f32,
    calibrate_before_scan: bool,
    serial_controller: Option<Arc<SerialController>>,
    camera_service: Option<Arc<CameraService>>,
    status_message: String,
    scan_positions: Vec<(f32, f32)>,
    dwell_time_ms: u64,
    capture_timeout_ms: u16,
    move_speed_mm_s: f32,
    fast_capture: bool,
    last_results: Option<Vec<ScanMeasurement>>,
}

impl State {
    pub fn new() -> Self {
        Self {
            is_scanning: false,
            scan_progress: 0.0,
            start_x: -1.0,
            start_y: -1.0,
            end_x: 1.0,
            end_y: 1.0,
            step_size: 0.1,
            scan_type: ScanType::Grid,
            radius: 0.5,
            angular_step: 10.0,
            center_x: 0.0,
            center_y: 0.0,
            calibrate_before_scan: true,
            serial_controller: None,
            camera_service: None,
            status_message: "Ready".to_string(),
            scan_positions: Vec::new(),
            dwell_time_ms: 250,
            capture_timeout_ms: 150,
            move_speed_mm_s: 5.0,
            fast_capture: true,
            last_results: None,
        }
    }

    pub fn set_serial_controller(&mut self, controller: Arc<SerialController>) {
        self.serial_controller = Some(controller);
    }

    pub fn clear_serial_controller(&mut self) {
        self.serial_controller = None;
    }

    pub fn set_camera_service(&mut self, service: Arc<CameraService>) {
        self.camera_service = Some(service);
    }

    pub fn clear_camera_service(&mut self) {
        self.camera_service = None;
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::StartScan => {
                if self.is_scanning {
                    self.status_message = "Scan already running".into();
                    return Command::none();
                }

                let controller = match self.serial_controller.clone() {
                    Some(controller) => controller,
                    None => {
                        self.status_message = "Controller not connected".into();
                        return Command::none();
                    }
                };

                let camera = match self.camera_service.clone() {
                    Some(camera) => camera,
                    None => {
                        self.status_message = "Camera not connected".into();
                        return Command::none();
                    }
                };

                self.scan_positions = self.generate_scan_positions();
                self.scan_progress = 0.0;
                self.last_results = None;

                if self.scan_positions.is_empty() {
                    self.status_message = "No scan positions generated".into();
                    return Command::none();
                }

                self.is_scanning = true;
                self.status_message =
                    format!("Scanning {} positions...", self.scan_positions.len());

                let dwell = Duration::from_millis(self.dwell_time_ms);
                let timeout = self.capture_timeout_ms;
                let speed = self.move_speed_mm_s;
                let calibrate = self.calibrate_before_scan;
                let positions = self.scan_positions.clone();
                let mode = self.current_camera_mode(&camera);

                return Command::perform(
                    execute_scan(
                        controller, camera, positions, mode, dwell, speed, calibrate, timeout,
                    ),
                    Message::ScanFinished,
                );
            }
            Message::StopScan => {
                self.is_scanning = false;
                self.status_message = "Scan stopped".to_string();
                Command::none()
            }
            Message::ScanProgress(progress) => {
                self.scan_progress = progress;
                Command::none()
            }
            Message::UpdateStartX(x_str) => {
                if let Ok(x) = x_str.parse::<f32>() {
                    self.start_x = x;
                }
                Command::none()
            }
            Message::UpdateStartY(y_str) => {
                if let Ok(y) = y_str.parse::<f32>() {
                    self.start_y = y;
                }
                Command::none()
            }
            Message::UpdateEndX(x_str) => {
                if let Ok(x) = x_str.parse::<f32>() {
                    self.end_x = x;
                }
                Command::none()
            }
            Message::UpdateEndY(y_str) => {
                if let Ok(y) = y_str.parse::<f32>() {
                    self.end_y = y;
                }
                Command::none()
            }
            Message::UpdateStepSize(step_str) => {
                if let Ok(step) = step_str.parse::<f32>() {
                    self.step_size = step;
                }
                Command::none()
            }
            Message::UpdateRadius(radius_str) => {
                if let Ok(radius) = radius_str.parse::<f32>() {
                    self.radius = radius;
                }
                Command::none()
            }
            Message::UpdateAngularStep(step_str) => {
                if let Ok(step) = step_str.parse::<f32>() {
                    self.angular_step = step;
                }
                Command::none()
            }
            Message::UpdateCenterX(x_str) => {
                if let Ok(x) = x_str.parse::<f32>() {
                    self.center_x = x;
                }
                Command::none()
            }
            Message::UpdateCenterY(y_str) => {
                if let Ok(y) = y_str.parse::<f32>() {
                    self.center_y = y;
                }
                Command::none()
            }
            Message::ScanTypeChanged(scan_type) => {
                self.scan_type = scan_type;
                Command::none()
            }
            Message::ToggleCalibrateBeforeScan(value) => {
                self.calibrate_before_scan = value;
                Command::none()
            }
            Message::UpdateDwellMs(value) => {
                if let Ok(ms) = value.parse::<u64>() {
                    self.dwell_time_ms = ms;
                }
                Command::none()
            }
            Message::UpdateCaptureTimeout(value) => {
                if let Ok(ms) = value.parse::<u16>() {
                    self.capture_timeout_ms = ms.max(50);
                }
                Command::none()
            }
            Message::UpdateMoveSpeed(value) => {
                if let Ok(speed) = value.parse::<f32>() {
                    self.move_speed_mm_s = speed.clamp(0.1, 50.0);
                }
                Command::none()
            }
            Message::ToggleFastCapture(value) => {
                self.fast_capture = value;
                Command::none()
            }
            Message::ScanFinished(result) => {
                self.is_scanning = false;
                match result {
                    Ok(measurements) => {
                        self.scan_progress = 1.0;
                        self.status_message =
                            format!("Scan complete: {} frames captured", measurements.len());
                        self.last_results = Some(measurements);
                    }
                    Err(err) => {
                        self.status_message = format!("Scan failed: {}", err);
                        self.last_results = None;
                    }
                }
                Command::none()
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let scan_bounds = row![
            column![
                text("Start X"),
                text_input("Start X", &self.start_x.to_string()).on_input(Message::UpdateStartX),
            ]
            .spacing(4),
            column![
                text("Start Y"),
                text_input("Start Y", &self.start_y.to_string()).on_input(Message::UpdateStartY),
            ]
            .spacing(4),
            column![
                text("End X"),
                text_input("End X", &self.end_x.to_string()).on_input(Message::UpdateEndX),
            ]
            .spacing(4),
            column![
                text("End Y"),
                text_input("End Y", &self.end_y.to_string()).on_input(Message::UpdateEndY),
            ]
            .spacing(4),
            column![
                text("Step"),
                text_input("Step", &self.step_size.to_string()).on_input(Message::UpdateStepSize),
            ]
            .spacing(4),
        ]
        .spacing(16);

        let geometry_controls = row![
            column![
                text("Scan Type"),
                pick_list(
                    vec![ScanType::Grid, ScanType::ConcentricCircle, ScanType::Spiral],
                    Some(self.scan_type.clone()),
                    Message::ScanTypeChanged,
                ),
            ]
            .spacing(4),
            column![
                text("Radius"),
                text_input("Radius", &self.radius.to_string()).on_input(Message::UpdateRadius),
            ]
            .spacing(4),
            column![
                text("Angular Step"),
                text_input("Angular", &self.angular_step.to_string())
                    .on_input(Message::UpdateAngularStep),
            ]
            .spacing(4),
            column![
                text("Center X"),
                text_input("Center X", &self.center_x.to_string()).on_input(Message::UpdateCenterX),
            ]
            .spacing(4),
            column![
                text("Center Y"),
                text_input("Center Y", &self.center_y.to_string()).on_input(Message::UpdateCenterY),
            ]
            .spacing(4),
        ]
        .spacing(16);

        let start_button = if self.is_scanning {
            button("Scanning...").style(iced::theme::Button::Primary)
        } else {
            button("Start Scan")
                .on_press(Message::StartScan)
                .style(iced::theme::Button::Primary)
        };

        let execution_controls = row![
            start_button,
            button("Stop Scan")
                .on_press(Message::StopScan)
                .style(iced::theme::Button::Destructive),
            checkbox("Calibrate before scan", self.calibrate_before_scan)
                .on_toggle(Message::ToggleCalibrateBeforeScan),
            checkbox("Fast capture", self.fast_capture).on_toggle(Message::ToggleFastCapture),
        ]
        .spacing(16);

        let timing_controls = row![
            column![
                text("Dwell (ms)"),
                text_input("Dwell", &self.dwell_time_ms.to_string())
                    .on_input(Message::UpdateDwellMs),
            ]
            .spacing(4),
            column![
                text("Capture timeout (ms)"),
                text_input("Timeout", &self.capture_timeout_ms.to_string())
                    .on_input(Message::UpdateCaptureTimeout),
            ]
            .spacing(4),
            column![
                text("Move speed (mm/s)"),
                text_input("Speed", &format!("{:.1}", self.move_speed_mm_s))
                    .on_input(Message::UpdateMoveSpeed),
            ]
            .spacing(4),
        ]
        .spacing(16);

        let mut layout = column![
            text("Scan Parameters"),
            scan_bounds,
            geometry_controls,
            execution_controls,
            timing_controls,
            progress_bar(0.0..=1.0, self.scan_progress),
            text(format!("Status: {}", self.status_message.clone())),
        ]
        .spacing(16);

        if let Some(results) = &self.last_results {
            let frames = results.len();
            let mut summary = column![text(format!(
                "Last scan captured {frames} frame{}",
                if frames == 1 { "" } else { "s" }
            ))]
            .spacing(4);

            if let Some(last) = results.last() {
                summary = summary.push(text(format!(
                    "Last frame span: {:.2} degC .. {:.2} degC",
                    last.frame.min_temp, last.frame.max_temp
                )));
            }

            layout = layout.push(summary);
        }

        layout.into()
    }

    // Generate positions for scanning based on the selected scan type
    fn generate_scan_positions(&self) -> Vec<(f32, f32)> {
        match self.scan_type {
            ScanType::Grid => self.generate_grid_positions(),
            ScanType::ConcentricCircle => self.generate_concentric_circle_positions(),
            ScanType::Spiral => self.generate_spiral_positions(),
        }
    }

    // Generate grid scan positions
    fn generate_grid_positions(&self) -> Vec<(f32, f32)> {
        let mut positions = Vec::new();
        let mut y = self.start_y;
        let mut row_count = 0;

        while y <= self.end_y {
            if row_count % 2 == 0 {
                // Even rows: left to right
                let mut x = self.start_x;
                while x <= self.end_x {
                    positions.push((x, y));
                    x += self.step_size;
                }
            } else {
                // Odd rows: right to left (for smooth motion)
                let mut x = self.end_x;
                while x >= self.start_x {
                    positions.push((x, y));
                    x -= self.step_size;
                }
            }
            y += self.step_size;
            row_count += 1;
        }

        positions
    }

    // Generate concentric circle scan positions
    fn generate_concentric_circle_positions(&self) -> Vec<(f32, f32)> {
        let mut positions = Vec::new();
        let angular_step_rad = self.angular_step.to_radians();
        let max_radius =
            ((self.end_x - self.start_x).abs() / 2.0).min((self.end_y - self.start_y).abs() / 2.0);
        let num_radii = (max_radius / self.step_size) as usize;

        for i in 1..=num_radii {
            let radius = i as f32 * self.step_size;
            if radius > max_radius {
                break;
            }

            let num_points = (2.0 * PI / angular_step_rad).ceil() as usize;
            for j in 0..num_points {
                let angle = j as f32 * angular_step_rad;
                let x = self.center_x + radius * angle.cos();
                let y = self.center_y + radius * angle.sin();

                // Only add positions within the defined bounds
                if x >= self.start_x && x <= self.end_x && y >= self.start_y && y <= self.end_y {
                    positions.push((x, y));
                }
            }
        }

        positions
    }

    // Generate spiral scan positions
    fn generate_spiral_positions(&self) -> Vec<(f32, f32)> {
        let mut positions = Vec::new();
        let angular_step_rad = self.angular_step.to_radians();
        let max_radius =
            ((self.end_x - self.start_x).abs() / 2.0).min((self.end_y - self.start_y).abs() / 2.0);
        let mut radius = self.step_size;
        let mut angle = 0.0f32;

        while radius <= max_radius {
            let x = self.center_x + radius * angle.cos();
            let y = self.center_y + radius * angle.sin();

            // Only add positions within the defined bounds
            if x >= self.start_x && x <= self.end_x && y >= self.start_y && y <= self.end_y {
                positions.push((x, y));
            }

            angle += angular_step_rad;
            radius += self.step_size * angular_step_rad / (2.0 * PI); // Adjust radius increment for smooth spiral
        }

        positions
    }

    fn current_camera_mode(&self, camera: &Arc<CameraService>) -> CameraMode {
        if self.fast_capture {
            CameraMode::Fast(camera.default_fast_settings())
        } else {
            CameraMode::Precise(camera.default_precise_settings())
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    StartScan,
    StopScan,
    ScanProgress(f32),
    UpdateStartX(String),
    UpdateStartY(String),
    UpdateEndX(String),
    UpdateEndY(String),
    UpdateStepSize(String),
    UpdateRadius(String),
    UpdateAngularStep(String),
    UpdateCenterX(String),
    UpdateCenterY(String),
    ScanTypeChanged(ScanType),
    ToggleCalibrateBeforeScan(bool),
    UpdateDwellMs(String),
    UpdateCaptureTimeout(String),
    UpdateMoveSpeed(String),
    ToggleFastCapture(bool),
    ScanFinished(Result<Vec<ScanMeasurement>, ScanError>),
}

pub fn view(state: &State) -> Element<'_, Message> {
    state.view()
}
