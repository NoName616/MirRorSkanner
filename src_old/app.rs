use eframe::egui;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::collections::VecDeque;

use crate::camera::{CameraController, CameraMode, TemperatureData, FlagState};
use crate::utils::{TemperatureUnit, ColorMap};

// Application state structure
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct AppState {
    pub connected: bool,
    pub camera_mode: CameraMode,
    pub temperature_unit: TemperatureUnit,
    pub color_map: ColorMap,
    pub brightness: f32,
    pub contrast: f32,
    pub zoom: f32,
    pub show_ir_image: bool,
    pub show_visible_image: bool,
    pub temperature_data: TemperatureData,
    pub flag_state: FlagState,
    pub frame_count: u64,
    #[serde(skip)]
    pub last_frame_time: Option<Instant>,
    pub library_version: String,
    pub error_message: Option<String>,
    #[serde(skip)]
    pub notifications: VecDeque<(String, Instant)>,
    #[serde(skip)]
    pub ir_image: egui::ColorImage,
    #[serde(skip)]
    pub visible_image: egui::ColorImage,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            connected: false,
            camera_mode: CameraMode::Callback,
            temperature_unit: TemperatureUnit::Celsius,
            color_map: ColorMap::Iron,
            brightness: 0.5,
            contrast: 1.0,
            zoom: 1.0,
            show_ir_image: true,
            show_visible_image: false,
            temperature_data: TemperatureData::default(),
            flag_state: FlagState::Unknown,
            frame_count: 0,
            last_frame_time: None,
            library_version: "v120".to_string(),
            error_message: None,
            notifications: VecDeque::new(),
            ir_image: egui::ColorImage::example(),
            visible_image: egui::ColorImage::example(),
        }
    }
}

pub struct CameraApp {
    state: Arc<Mutex<AppState>>,
    controller: Option<CameraController>,
    last_update: Instant,
    ir_image: Option<egui::TextureHandle>,
    visible_image: Option<egui::TextureHandle>,
}

impl CameraApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load previous state from storage if available
        let state = if let Some(storage) = cc.storage {
            storage.get_string(eframe::APP_KEY).map(|s| serde_json::from_str(&s).unwrap_or_default()).unwrap_or_default()
        } else {
            Default::default()
        };

        let state = Arc::new(Mutex::new(state));
        
        Self {
            state: state.clone(),
            controller: None,
            last_update: Instant::now(),
            ir_image: None,
            visible_image: None,
        }
    }

    fn add_notification(&mut self, message: String) {
        let mut state = self.state.lock().unwrap();
        state.notifications.push_back((message, Instant::now()));
        // Keep only the last 10 notifications
        if state.notifications.len() > 10 {
            state.notifications.pop_front();
        }
    }

    fn connect_camera(&mut self) {
        let library_version = {
            let state = self.state.lock().unwrap();
            state.library_version.clone()
        };
        match CameraController::new(library_version) {
            Ok(mut controller) => {
                match controller.connect() {
                    Ok(_) => {
                        self.controller = Some(controller);
                        {
                            let mut state = self.state.lock().unwrap();
                            state.connected = true;
                            state.error_message = None;
                        }
                        self.add_notification("Camera connected successfully".to_string());
                    }
                    Err(e) => {
                        self.controller = None;
                        {
                            let mut state = self.state.lock().unwrap();
                            state.connected = false;
                            state.error_message = Some(format!("Connection failed: {}", e));
                        }
                        self.add_notification(format!("Connection failed: {}", e));
                    }
                }
            }
            Err(e) => {
                self.controller = None;
                {
                    let mut state = self.state.lock().unwrap();
                    state.connected = false;
                    state.error_message = Some(format!("Initialization failed: {}", e));
                }
                self.add_notification(format!("Initialization failed: {}", e));
            }
        }
    }

    fn disconnect_camera(&mut self) {
        if let Some(mut controller) = self.controller.take() {
            let _ = controller.disconnect(); // Ignore disconnect errors
            {
                let mut state = self.state.lock().unwrap();
                state.connected = false;
            }
            self.add_notification("Camera disconnected".to_string());
        }
    }

    fn start_recording(&mut self) {
        if let Some(ref controller) = self.controller {
            match controller.start_recording() {
                Ok(_) => {
                    self.add_notification("Recording started".to_string());
                }
                Err(e) => {
                    self.add_notification(format!("Recording start failed: {}", e));
                }
            }
        }
    }

    fn stop_recording(&mut self) {
        if let Some(ref controller) = self.controller {
            match controller.stop_recording() {
                Ok(_) => {
                    self.add_notification("Recording stopped".to_string());
                }
                Err(e) => {
                    self.add_notification(format!("Recording stop failed: {}", e));
                }
            }
        }
    }

    fn capture_snapshot(&mut self) {
        if let Some(ref controller) = self.controller {
            match controller.capture_snapshot() {
                Ok(_) => {
                    self.add_notification("Snapshot captured".to_string());
                }
                Err(e) => {
                    self.add_notification(format!("Snapshot failed: {}", e));
                }
            }
        }
    }

    fn capture_screenshot(&mut self) {
        if let Some(ref controller) = self.controller {
            match controller.capture_screenshot() {
                Ok(_) => {
                    self.add_notification("Screenshot captured".to_string());
                }
                Err(e) => {
                    self.add_notification(format!("Screenshot failed: {}", e));
                }
            }
        }
    }

    fn set_camera_mode(&mut self, mode: CameraMode) {
        if let Some(ref mut controller) = self.controller {
            match controller.set_mode(mode) {
                Ok(_) => {
                    {
                        let mut state = self.state.lock().unwrap();
                        state.camera_mode = mode;
                    }
                    self.add_notification(format!("Mode changed to {:?}", mode));
                }
                Err(e) => {
                    self.add_notification(format!("Mode change failed: {}", e));
                }
            }
        }
    }

    fn render_connection_panel(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("Camera Connection");
            
            let state_clone = {
                let state = self.state.lock().unwrap();
                (state.connected, state.library_version.clone(), state.error_message.clone())
            };
            
            let (connected, library_version, error_message) = state_clone;
            
            ui.horizontal(|ui| {
                if ui.add_enabled(!connected, egui::Button::new("Connect")).clicked() {
                    self.connect_camera();
                }
                
                if ui.add_enabled(connected, egui::Button::new("Disconnect")).clicked() {
                    self.disconnect_camera();
                }
                
                ui.label(if connected { "🟢 Connected" } else { "🔴 Disconnected" });
            });
            
            ui.separator();
            
            ui.horizontal(|ui| {
                ui.label("Library Version:");
                let mut new_version = library_version.clone();
                egui::ComboBox::from_label("")
                    .selected_text(&new_version)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut new_version, "v90".to_string(), "v90");
                        ui.selectable_value(&mut new_version, "v100".to_string(), "v100");
                        ui.selectable_value(&mut new_version, "v120".to_string(), "v120");
                    });
                    
                // Update the version if changed
                if new_version != library_version {
                    let mut state = self.state.lock().unwrap();
                    state.library_version = new_version;
                }
            });
            
            if let Some(ref error) = error_message {
                ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
            }
        });
    }

    fn render_modes_panel(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("Operation Mode");
            
            let state_clone = {
                let state = self.state.lock().unwrap();
                (state.camera_mode, state.temperature_unit)
            };
            
            let (mut camera_mode, mut temperature_unit) = state_clone;
            
            ui.horizontal(|ui| {
                if ui.radio_value(&mut camera_mode, CameraMode::Callback, "Callback").clicked() {
                    self.set_camera_mode(CameraMode::Callback);
                }
                
                if ui.radio_value(&mut camera_mode, CameraMode::Polling, "Polling").clicked() {
                    self.set_camera_mode(CameraMode::Polling);
                }
            });
            
            ui.separator();
            
            ui.horizontal(|ui| {
                ui.label("Temperature Unit:");
                egui::ComboBox::from_label("")
                    .selected_text(format!("{:?}", temperature_unit))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut temperature_unit, TemperatureUnit::Celsius, "Celsius");
                        ui.selectable_value(&mut temperature_unit, TemperatureUnit::Fahrenheit, "Fahrenheit");
                        ui.selectable_value(&mut temperature_unit, TemperatureUnit::Kelvin, "Kelvin");
                    });
            });
            
            // Update state if values changed
            if camera_mode != state_clone.0 || temperature_unit != state_clone.1 {
                let mut state = self.state.lock().unwrap();
                if camera_mode != state.camera_mode {
                    state.camera_mode = camera_mode;
                }
                if temperature_unit != state.temperature_unit {
                    state.temperature_unit = temperature_unit;
                }
            }
        });
    }

    fn render_image_display(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("Image Display");
            
            let state_clone = {
                let state = self.state.lock().unwrap();
                (state.show_ir_image, state.show_visible_image, state.color_map, state.brightness, state.contrast, state.zoom)
            };
            
            let (mut show_ir_image, mut show_visible_image, mut color_map, mut brightness, mut contrast, mut zoom) = state_clone;
            
            ui.horizontal(|ui| {
                ui.checkbox(&mut show_ir_image, "IR Image")
                    .on_hover_text("Show infrared thermal image");
                ui.checkbox(&mut show_visible_image, "Visible Image")
                    .on_hover_text("Show visible light image");
            });
            
            ui.separator();
            
            ui.horizontal(|ui| {
                ui.label("Color Map:");
                egui::ComboBox::from_label("")
                    .selected_text(format!("{:?}", color_map))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut color_map, ColorMap::Iron, "Iron");
                        ui.selectable_value(&mut color_map, ColorMap::Rainbow, "Rainbow");
                        ui.selectable_value(&mut color_map, ColorMap::Grayscale, "Grayscale");
                        ui.selectable_value(&mut color_map, ColorMap::Red, "Red");
                        ui.selectable_value(&mut color_map, ColorMap::Blue, "Blue");
                    });
            });
            
            ui.add(egui::Slider::new(&mut brightness, 0.0..=1.0).text("Brightness"));
            ui.add(egui::Slider::new(&mut contrast, 0.5..=2.0).text("Contrast"));
            ui.add(egui::Slider::new(&mut zoom, 0.1..=5.0).text("Zoom"));
            
            ui.separator();
            
            // Display images
            if show_ir_image {
                if let Some(ref ir_image) = self.ir_image {
                    let available_size = ui.available_size();
                    let desired_size = egui::vec2(
                        available_size.x.min(ir_image.size()[0] as f32 * zoom),
                        available_size.y.min(ir_image.size()[1] as f32 * zoom) * 0.4, // Half height for IR image
                    );
                    ui.image((ir_image.id(), desired_size))
                        .on_hover_text("Infrared thermal image");
                } else {
                    ui.label("No IR image available");
                }
            }
            
            if show_visible_image {
                if let Some(ref visible_image) = self.visible_image {
                    let available_size = ui.available_size();
                    let desired_size = egui::vec2(
                        available_size.x.min(visible_image.size()[0] as f32 * zoom),
                        available_size.y.min(visible_image.size()[1] as f32 * zoom) * 0.4, // Half height for visible image
                    );
                    ui.image((visible_image.id(), desired_size))
                        .on_hover_text("Visible light image");
                } else {
                    ui.label("No visible image available");
                }
            }
            
            // Update state if values changed
            if show_ir_image != state_clone.0 || show_visible_image != state_clone.1 || color_map != state_clone.2 ||
               brightness != state_clone.3 || contrast != state_clone.4 || zoom != state_clone.5 {
                let mut state = self.state.lock().unwrap();
                state.show_ir_image = show_ir_image;
                state.show_visible_image = show_visible_image;
                state.color_map = color_map;
                state.brightness = brightness;
                state.contrast = contrast;
                state.zoom = zoom;
            }
        });
    }

    fn render_parameters_panel(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("Camera Parameters");
            
            let state = self.state.lock().unwrap();
            
            ui.label(format!("Chip Temperature: {:.2}°{:?}",
                convert_temperature(state.temperature_data.chip_temp, state.temperature_unit),
                state.temperature_unit));
            ui.label(format!("Flag Temperature: {:.2}°{:?}",
                convert_temperature(state.temperature_data.flag_temp, state.temperature_unit),
                state.temperature_unit));
            ui.label(format!("Housing Temperature: {:.2}°{:?}",
                convert_temperature(state.temperature_data.housing_temp, state.temperature_unit),
                state.temperature_unit));
            
            ui.separator();
            
            ui.label(format!("Flag State: {:?}", state.flag_state));
            ui.label(format!("Frame Count: {}", state.frame_count));
            
            if let Some(last_frame_time) = state.last_frame_time {
                let elapsed = last_frame_time.elapsed().as_millis();
                ui.label(format!("Last Frame: {} ms ago", elapsed));
            } else {
                ui.label("No frames received yet");
            }
        });
    }

    fn render_tools_panel(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("Tools");
            
            ui.horizontal(|ui| {
                if ui.add_enabled(self.controller.is_some(), egui::Button::new("Snapshot")).clicked() {
                    self.capture_snapshot();
                }
                
                if ui.add_enabled(self.controller.is_some(), egui::Button::new("Screenshot")).clicked() {
                    self.capture_screenshot();
                }
                
                if ui.add_enabled(self.controller.is_some(), egui::Button::new("Start Record")).clicked() {
                    self.start_recording();
                }
                
                if ui.add_enabled(self.controller.is_some(), egui::Button::new("Stop Record")).clicked() {
                    self.stop_recording();
                }
            });
        });
    }

    fn render_notifications(&mut self, ctx: &egui::Context) {
        let mut state = self.state.lock().unwrap();
        
        // Remove notifications older than 5 seconds
        state.notifications.retain(|(_, time)| time.elapsed() < Duration::from_secs(5));
        
        let notifications: Vec<_> = state.notifications.clone().into();
        drop(state);
        
        egui::TopBottomPanel::bottom("notifications")
            .resizable(false)
            .min_height(20.0)
            .max_height(150.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    for (msg, _) in &notifications {
                        ui.label(msg);
                    }
                });
            });
    }
}

fn convert_temperature(temp_celsius: f32, unit: TemperatureUnit) -> f32 {
    match unit {
        TemperatureUnit::Celsius => temp_celsius,
        TemperatureUnit::Fahrenheit => temp_celsius * 9.0 / 5.0 + 32.0,
        TemperatureUnit::Kelvin => temp_celsius + 273.15,
    }
}

fn convert_temperature_to_color_image(
    temp_data: &[f32],
    color_map: &ColorMap,
    brightness: f32,
    contrast: f32
) -> Result<egui::ColorImage, String> {
    if temp_data.is_empty() {
        return Err("Empty temperature data".to_string());
    }
    
    // Find min and max temperatures for normalization
    let min_temp = temp_data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
    let max_temp = temp_data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    let temp_range = if max_temp == min_temp { 1.0 } else { max_temp - min_temp };
    
    // Convert to RGB based on color map
    let mut rgba_pixels: Vec<u8> = Vec::new();
    for &temp in temp_data {
        let normalized = ((temp - min_temp) / temp_range).clamp(0.0, 1.0);
        
        // Apply contrast and brightness
        let adjusted = ((normalized - 0.5) * contrast + 0.5).clamp(0.0, 1.0);
        let brightness_applied = (adjusted + brightness - 0.5).clamp(0.0, 1.0);
        
        let color = match color_map {
            ColorMap::Iron => {
                let r = (brightness_applied * 255.0).min(255.0) as u8;
                let g = (brightness_applied * brightness_applied * 255.0).min(255.0) as u8;
                let b = (brightness_applied * brightness_applied * 128.0).min(255.0) as u8;
                [r, g, b, 255]
            }
            ColorMap::Rainbow => {
                let hue = brightness_applied * 240.0; // 0-240 for red to blue
                let hsv = egui::ecolor::Hsva::new(hue / 360.0, 1.0, brightness_applied, 1.0);
                let rgba: [u8; 4] = egui::Color32::from(hsv).to_array();
                rgba
            }
            ColorMap::Grayscale => {
                let gray = (brightness_applied * 255.0).min(255.0) as u8;
                [gray, gray, gray, 255]
            }
            ColorMap::Red => {
                let r = (brightness_applied * 255.0).min(255.0) as u8;
                [r, 0, 0, 255]
            }
            ColorMap::Blue => {
                let b = (brightness_applied * 255.0).min(255.0) as u8;
                [0, 0, b, 255]
            }
        };
        
        rgba_pixels.extend_from_slice(&color);
    }
    
    // Assuming a square image for simplicity (adjust dimensions as needed)
    let size = (temp_data.len() as f32).sqrt().round() as usize;
    if size * size != temp_data.len() {
        // If not a perfect square, use a common thermal camera resolution (e.g., 640x480 = 30720)
        // For now, just use a reasonable approximation
        let width = 640;
        let height = (temp_data.len() + width - 1) / width; // Ceiling division
        
        Ok(egui::ColorImage::from_rgba_unmultiplied([width, height], &rgba_pixels))
    } else {
        Ok(egui::ColorImage::from_rgba_unmultiplied([size, size], &rgba_pixels))
    }
}

fn convert_u8_to_color_image(data: &[u8]) -> Result<egui::ColorImage, String> {
    if data.is_empty() {
        return Err("Empty image data".to_string());
    }
    
    // Assuming grayscale or RGB data
    // For simplicity, assuming it's RGB (3 bytes per pixel) or grayscale
    if data.len() % 3 != 0 {
        // If not RGB, assume grayscale and convert to RGBA
        let mut rgba_pixels: Vec<u8> = Vec::with_capacity(data.len() * 4);
        for &gray in data {
            rgba_pixels.extend_from_slice(&[gray, gray, gray, 255]);
        }
        
        // Assuming a square image for simplicity (adjust dimensions as needed)
        let size = (data.len() as f32).sqrt().round() as usize;
        if size * size != data.len() {
            // Use common resolution if not square
            let width = 640;
            let height = (data.len() + width - 1) / width;
            Ok(egui::ColorImage::from_rgba_unmultiplied([width, height], &rgba_pixels))
        } else {
            Ok(egui::ColorImage::from_rgba_unmultiplied([size, size], &rgba_pixels))
        }
    } else {
        // RGB data - convert to RGBA
        let mut rgba_pixels: Vec<u8> = Vec::with_capacity(data.len() * 4/3); // 4 bytes per pixel instead of 3
        for chunk in data.chunks(3) {
            if chunk.len() == 3 {
                rgba_pixels.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
            }
        }
        
        let width = 640; // Common width for visible image
        let height = (data.len() / 3 + width - 1) / width; // Calculate height based on RGB
        
        Ok(egui::ColorImage::from_rgba_unmultiplied([width, height], &rgba_pixels))
    }
}

impl eframe::App for CameraApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update camera state periodically (every 100ms)
        if self.last_update.elapsed() > Duration::from_millis(100) {
            // Get camera state and update images
            if let Some(ref controller) = self.controller {
                if let Ok(camera_state) = controller.get_camera_state() {
                    let mut state = self.state.lock().unwrap();
                    
                    state.temperature_data = camera_state.temperature_data;
                    state.flag_state = camera_state.flag_state;
                    state.frame_count = camera_state.frame_count;
                    state.last_frame_time = Some(Instant::now());
                    
                    // Update images if available - store in CameraApp and update state
                    if let Some(ir_data) = &camera_state.ir_image {
                        if let Ok(color_image) = convert_temperature_to_color_image(&ir_data.data, &state.color_map, state.brightness, state.contrast) {
                            let texture = ctx.load_texture("ir_image", color_image.clone(), egui::TextureOptions::default());
                            self.ir_image = Some(texture);
                            
                            // Also update the state with the color image
                            let mut app_state = self.state.lock().unwrap();
                            app_state.ir_image = color_image;
                        }
                    }
                    
                    if let Some(visible_data) = &camera_state.visible_image {
                        if let Ok(color_image) = convert_u8_to_color_image(&visible_data.data) {
                        let texture = ctx.load_texture("visible_image", color_image.clone(), egui::TextureOptions::default());
                        self.visible_image = Some(texture);
                        
                        // Also update the state with the color image
                        let mut app_state = self.state.lock().unwrap();
                        app_state.visible_image = color_image;
                    }
                }
            }
        }
            self.last_update = Instant::now();
        }
        
        // Main window
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        ui.close_menu();
                    }
                });
                
                ui.menu_button("View", |ui| {
                    let mut state = self.state.lock().unwrap();
                    ui.checkbox(&mut state.show_ir_image, "IR Image");
                    ui.checkbox(&mut state.show_visible_image, "Visible Image");
                    ui.separator();
                    ui.label("Color Maps:");
                    ui.selectable_value(&mut state.color_map, ColorMap::Iron, "Iron");
                    ui.selectable_value(&mut state.color_map, ColorMap::Rainbow, "Rainbow");
                    ui.selectable_value(&mut state.color_map, ColorMap::Grayscale, "Grayscale");
                    ui.selectable_value(&mut state.color_map, ColorMap::Red, "Red");
                    ui.selectable_value(&mut state.color_map, ColorMap::Blue, "Blue");
                    drop(state);
                });
                
                ui.menu_button("Help", |ui| {
                    ui.label("Optris Pi 640 Camera Control");
                    ui.label("Version 1.0");
                });
            });
        });
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Left panel - Controls
                ui.vertical(|ui| {
                    ui.set_min_width(300.0);
                    
                    self.render_connection_panel(ui);
                    ui.add_space(10.0);
                    
                    self.render_modes_panel(ui);
                    ui.add_space(10.0);
                    
                    self.render_parameters_panel(ui);
                    ui.add_space(10.0);
                    
                    self.render_tools_panel(ui);
                });
                
                // Right panel - Image display
                ui.vertical(|ui| {
                    self.render_image_display(ui);
                });
            });
        });
        
        // Render notifications at the bottom
        self.render_notifications(ctx);
        
        // Request repaint to update the UI continuously
        ctx.request_repaint();
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if let Ok(state) = self.state.lock() {
            if let Ok(json) = serde_json::to_string(&*state) {
                storage.set_string(eframe::APP_KEY, json);
            }
        }
    }
}