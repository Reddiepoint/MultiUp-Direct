use eframe::egui::{Context, ScrollArea, Ui};
use eframe::egui::Direction::TopDown;
use eframe::egui;
use egui::Align2;
use egui_toast::Toasts;

#[derive(Default)]
pub struct ExtractUI {
    toasts: Toasts,
    hosting_links: String,
}

impl ExtractUI {
    pub fn display(ctx: &Context, ui: &mut Ui, extract_ui: &mut ExtractUI) {
        extract_ui.toasts = Toasts::new()
            .anchor(Align2::RIGHT_TOP, (10.0, 10.0))
            .direction(TopDown);

        extract_ui.toasts.show(ctx);
        extract_ui.display_input_area(ui);
        extract_ui.display_footer(ctx);
        extract_ui.display_output_area(ui);

    }

    fn display_input_area(&mut self, ui: &mut Ui) {
    }

    pub fn display_error_log(&mut self, ctx: &Context) {
    }

    fn display_output_area(&mut self, ui: &mut Ui) {
    }

    fn display_footer(&mut self, ctx: &Context) {
    }
}