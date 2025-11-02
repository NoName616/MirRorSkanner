use iced::widget::{button, column, progress_bar, row, text, text_input, pick_list, checkbox};
use iced::Element;
use std::f32::consts::PI;
use std::sync::{Arc, Mutex};

use crate::hardware::serial_communication::{SerialController, ControllerCommand};

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
    serial_controller: Option<Arc<Mutex<SerialController>>>,
    status_message: String,
    scan_positions: Vec<(f32, f32)>,
    current_position_index: usize,
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
            status_message: "Ready".to_string(),
            scan_positions: Vec::new(),
            current_position_index: 0,
        }
    }

    pub fn set_serial_controller(&mut self, controller: Arc<Mutex<SerialController>>) {
        self.serial_controller = Some(controller);
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::StartScan => {
                // Generate scan positions based on selected scan type
                self.scan_positions = self.generate_scan_positions();
                self.current_position_index = 0;
                self.scan_progress = 0.0;
                self.is_scanning = true;
                self.status_message = "Scanning...".to_string();
                
                // If we have a controller and need to calibrate
                if let Some(ref controller) = self.serial_controller {
                    if self.calibrate_before_scan {
                        let _ = controller.lock().unwrap().send_controller_command(ControllerCommand::Home);
                        // In a real implementation, we would wait for calibration to complete
                        // before starting the scan
                    }
                    
                    // Move to the first position if we have positions
                    if !self.scan_positions.is_empty() {
                        let (x, _y) = self.scan_positions[0];
                        // Используем стандартную скорость 5 мм/с для сканирования
                        let _ = controller.lock().unwrap().send_controller_command(
                            ControllerCommand::Move { x_mm: x, speed_mm_s: 5.0 }
                        );
                        self.current_position_index = 1;
                        if self.scan_positions.len() > 0 {
                            self.scan_progress = 1.0 / self.scan_positions.len() as f32;
                        }
                    }
                }
            }
            Message::StopScan => {
                self.is_scanning = false;
                self.status_message = "Scan stopped".to_string();
            }
            Message::ScanProgress(progress) => {
                self.scan_progress = progress;
            }
            Message::UpdateStartX(x_str) => {
                if let Ok(x) = x_str.parse::<f32>() {
                    self.start_x = x;
                }
            }
            Message::UpdateStartY(y_str) => {
                if let Ok(y) = y_str.parse::<f32>() {
                    self.start_y = y;
                }
            }
            Message::UpdateEndX(x_str) => {
                if let Ok(x) = x_str.parse::<f32>() {
                    self.end_x = x;
                }
            }
            Message::UpdateEndY(y_str) => {
                if let Ok(y) = y_str.parse::<f32>() {
                    self.end_y = y;
                }
            }
            Message::UpdateStepSize(step_str) => {
                if let Ok(step) = step_str.parse::<f32>() {
                    self.step_size = step;
                }
            }
            Message::UpdateRadius(radius_str) => {
                if let Ok(radius) = radius_str.parse::<f32>() {
                    self.radius = radius;
                }
            }
            Message::UpdateAngularStep(step_str) => {
                if let Ok(step) = step_str.parse::<f32>() {
                    self.angular_step = step;
                }
            }
            Message::UpdateCenterX(x_str) => {
                if let Ok(x) = x_str.parse::<f32>() {
                    self.center_x = x;
                }
            }
            Message::UpdateCenterY(y_str) => {
                if let Ok(y) = y_str.parse::<f32>() {
                    self.center_y = y;
                }
            }
            Message::ScanTypeChanged(scan_type) => {
                self.scan_type = scan_type;
            }
            Message::ToggleCalibrateBeforeScan(value) => {
                self.calibrate_before_scan = value;
            }
        }
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
        let max_radius = ((self.end_x - self.start_x).abs() / 2.0).min((self.end_y - self.start_y).abs() / 2.0);
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
        let max_radius = ((self.end_x - self.start_x).abs() / 2.0).min((self.end_y - self.start_y).abs() / 2.0);
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
}

pub fn view(state: &State) -> Element<'_, Message> {
    let scan_type_picklist = pick_list(
        [ScanType::Grid, ScanType::ConcentricCircle, ScanType::Spiral],
        Some(state.scan_type.clone()),
        Message::ScanTypeChanged,
    );

    let params = column![
        text("Scan Parameters"),
        row![
            text("Scan Type:").width(iced::Length::FillPortion(1)),
            scan_type_picklist.width(iced::Length::FillPortion(2)),
        ].spacing(10),
        text_input("Start X", &format!("{:.2}", state.start_x)).on_input(Message::UpdateStartX),
        text_input("Start Y", &format!("{:.2}", state.start_y)).on_input(Message::UpdateStartY),
        text_input("End X", &format!("{:.2}", state.end_x)).on_input(Message::UpdateEndX),
        text_input("End Y", &format!("{:.2}", state.end_y)).on_input(Message::UpdateEndY),
        text_input("Step Size", &format!("{:.2}", state.step_size)).on_input(Message::UpdateStepSize),
    ]
    .spacing(10);

    // Additional parameters for concentric circle and spiral scans
    let additional_params = if state.scan_type == ScanType::ConcentricCircle || state.scan_type == ScanType::Spiral {
        column![
            text("Circle/Spiral Parameters"),
            text_input("Radius", &format!("{:.2}", state.radius)).on_input(Message::UpdateRadius),
            text_input("Angular Step (deg)", &format!("{:.2}", state.angular_step)).on_input(Message::UpdateAngularStep),
            text_input("Center X", &format!("{:.2}", state.center_x)).on_input(Message::UpdateCenterX),
            text_input("Center Y", &format!("{:.2}", state.center_y)).on_input(Message::UpdateCenterY),
        ].spacing(10)
    } else {
        column![].spacing(10)
    };

    let controls = column![
        button("Start Scan").on_press(Message::StartScan),
        button("Stop Scan").on_press(Message::StopScan),
        checkbox("Calibrate before scan", state.calibrate_before_scan).on_toggle(Message::ToggleCalibrateBeforeScan),
        text(format!("Status: {}", state.status_message)),
        text(format!("Progress: {:.0}%", state.scan_progress * 100.0)),
        progress_bar(0.0..=1.0, state.scan_progress),
        text(format!("Total positions: {}", state.scan_positions.len())),
    ]
    .spacing(10);

    let layout = column![
        params,
        additional_params,
        controls,
    ]
    .spacing(10)
    .padding(10);

    layout.into()
}