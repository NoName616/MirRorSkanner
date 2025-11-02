mod app;
mod camera;
mod ffi;
mod utils;

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([400.0, 300.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Optris Pi 640 Camera Control",
        options,
        Box::new(|cc| Ok(Box::new(app::CameraApp::new(cc)))),
    )
}