use eframe::egui::{Align2, Context, Ui};
use eframe::egui::Direction::TopDown;
use egui_toast::Toasts;

#[derive(Default)]
pub struct UploadUI {
    pub toasts: Toasts,
}

impl UploadUI {
    pub fn display(ctx: &Context, ui: &mut Ui, upload_ui: &mut UploadUI) {
        upload_ui.toasts = Toasts::new()
            .anchor(Align2::RIGHT_TOP, (10.0, 10.0))
            .direction(TopDown);

        upload_ui.toasts.show(ctx);
    }
}
