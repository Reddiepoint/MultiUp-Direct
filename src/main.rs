#![windows_subsystem = "windows"]

use eframe::egui;

use crate::modules::main::Application;

//mod constants;
//mod functions;
mod modules;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        drag_and_drop_support: true,
        initial_window_size: Some(egui::Vec2::new(1280.0, 720.0)),
        resizable: true,
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "MultiUp Direct",
        options,
        Box::new(|_cc| Box::<Application>::default()),
    )
}
