use eframe::egui;
use eframe::egui::{Context, ScrollArea, TextEdit, Ui};
use eframe::egui::Direction::TopDown;
use egui::Align2;
use egui_toast::Toasts;
use crate::modules::hosting_services::general::extract_raw_links;

#[derive(Default)]
pub struct ExtractUI {
    toasts: Toasts,
    // Input
    hosting_links: String,
    recheck_validity: bool,
    extracting: bool,
    // Output
    search_query: String,
    // Footer
}

impl ExtractUI {
    pub fn display(ctx: &Context, ui: &mut Ui, extract_ui: &mut ExtractUI) {
        extract_ui.toasts = Toasts::new()
            .anchor(Align2::RIGHT_TOP, (10.0, 10.0))
            .direction(TopDown);

        extract_ui.toasts.show(ctx);
        extract_ui.display_input_area(ui);
        ui.separator();
        extract_ui.display_footer(ctx);
        extract_ui.display_output_area(ui);
    }

    fn display_input_area(&mut self, ui: &mut Ui) {
        ui.heading("Hosting Service Input Links");

        ScrollArea::both()
            .id_source("Hosting Service Input Links Box")
            .max_height(ui.available_height() / 4.0)
            .show(ui, |ui| {
                ui.add(
                    TextEdit::multiline(&mut self.hosting_links)
                        .hint_text("Enter your links here")
                        .desired_width(ui.available_width())
                );
            });

        ui.horizontal(|ui| {
            // Recheck validity checkbox
            ui.checkbox(&mut self.recheck_validity, "Recheck validity");
            
            // Extract button
            if ui.add_enabled(!self.extracting, egui::Button::new("Extract")).clicked() {
                self.extracting = true;
                extract_raw_links(self.hosting_links.clone());
            }
            
            // Extracting status
            if self.extracting {
                ui.spinner();
                ui.label("Extracting links...");
            }
        });
    }


    fn display_output_area(&mut self, ui: &mut Ui) {
        ui.heading("Extracted Links");
        
        // Filename search
        ui.horizontal(|ui| {
            ui.add(TextEdit::singleline(&mut self.search_query)
                .id_source("Filename Search Filter")
                .hint_text("Type to search for specific files"));

            if ui.button("Clear").clicked() {
                self.search_query = String::new();
            }
        });
        
        // todo!("Write output links")
    }

    fn display_footer(&mut self, ctx: &Context) {}
    pub fn display_error_log(&mut self, ctx: &Context) {}
}
