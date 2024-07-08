use eframe::egui::{Align2, Context, Ui};
use eframe::egui::Direction::TopDown;
use egui_toast::Toasts;

#[derive(Default)]
pub struct DebridUI {
    toasts: Toasts,
}

impl DebridUI {
    pub fn display(ctx: &Context, ui: &mut Ui, debrid_ui: &mut DebridUI) {
        debrid_ui.toasts = Toasts::new()
            .anchor(Align2::RIGHT_TOP, (10.0, 10.0))
            .direction(TopDown);

        debrid_ui.display_input_area(ui);
        debrid_ui.display_debrid_links_area(ui);

        debrid_ui.toasts.show(ctx);
    }

    fn display_input_area(&mut self, ui: &mut Ui) {}
    pub fn display_error_log(&mut self, ctx: &Context) {}
    fn display_debrid_links_area(&mut self, ui: &mut Ui) {}
}