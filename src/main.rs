#![windows_subsystem = "windows"]

mod functions;
mod structs;

use eframe::egui;
use crate::structs::main::Application;

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
