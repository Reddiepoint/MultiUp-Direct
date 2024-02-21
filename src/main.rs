#![windows_subsystem = "windows"]
mod modules;

use eframe::egui::ViewportBuilder;
use crate::modules::app::MultiUpDirect;


fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_drag_and_drop(true)
            .with_resizable(true)
            .with_inner_size((1280.0, 800.0)),
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "MultiUp Direct",
        options,
        Box::new(|_cc| Box::<MultiUpDirect>::default()),
    )
}
