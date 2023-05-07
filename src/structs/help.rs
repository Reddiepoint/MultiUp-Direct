use eframe::egui;
use eframe::egui::{Context};
use crate::constants::about::HELP_MESSAGE;

pub struct Help {}

impl Help {
    pub fn show(ctx: &Context, open: &mut bool) {
        egui::Window::new("Help").open(open).show(ctx, |ui| {
            ui.label(HELP_MESSAGE)
        });
    }
}
