use std::fs;
use eframe::egui::{Align2, Context, ScrollArea, TextEdit, Ui};
use eframe::egui::Direction::TopDown;
use egui_toast::{Toast, ToastKind, ToastOptions, Toasts};
use crate::modules::upload::UploadUI;

#[derive(Default)]
struct Channels {}

#[derive(Default)]
pub struct DebridUI {
    toasts: Toasts,
    channels: Channels,
    api_key: String,
    input_links: String,
    debrid_links: String,
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

    fn display_input_area(&mut self, ui: &mut Ui) {
        let input_area_height = ui.available_height() / 2.0;
        ui.set_max_height(input_area_height);
        ScrollArea::both()
            .id_source("Link Input Area")
            .max_height(input_area_height)
            .show(ui, |ui| {
                ui.add(
                    TextEdit::multiline(&mut self.input_links)
                        .hint_text("Paste your links here")
                        .desired_width(ui.available_width()),
                );
            });

        ui.horizontal(|ui| {
            ui.label("API Key:");
            ui.add(TextEdit::singleline(&mut self.api_key)
                .hint_text("Enter your API key here"));

            if ui.button("Read from file").clicked() {
                let api_key = fs::read_to_string("./api_key.txt");
                match api_key {
                    Ok(key) => {
                        self.api_key = key.trim().to_string();
                        self.toasts.add(Toast {
                            text: "Successfully read API key".into(),
                            kind: ToastKind::Success,
                            options: ToastOptions::default()
                                .duration_in_seconds(5.0)
                                .show_progress(true)
                                .show_icon(true)
                        });
                    }
                    Err(_) => {
                        self.toasts.add(Toast {
                            text: "Failed to read \"api_key.txt\"".into(),
                            kind: ToastKind::Error,
                            options: ToastOptions::default()
                                .duration_in_seconds(5.0)
                                .show_progress(true)
                                .show_icon(true)
                        });
                    },
                }
            }
        });

        if ui.button("Unlock links").clicked() {}
    }

    fn display_debrid_links_area(&mut self, ui: &mut Ui) {
        ui.heading("Debrid Links");
        let mut debrid_links = self.debrid_links.clone();
        ui.horizontal(|ui| {
            let copy_normal_button = ui.button("Copy");

            if copy_normal_button.clicked() {
                self.toasts.add(Toast {
                    text: "Copied debrid links".into(),
                    kind: ToastKind::Info,
                    options: ToastOptions::default()
                        .duration_in_seconds(5.0)
                        .show_progress(true)
                        .show_icon(true)
                });
            }

            let copy_quote_button = ui.button("Copy as \"{URL}\"");
            if copy_quote_button.hovered() {
                let mut new_debrid_links = String::new();
                let links = debrid_links.split("\n");
                for link in links {
                    new_debrid_links = format!("{}\"{}\"\n", new_debrid_links, link);
                }
                debrid_links = new_debrid_links
            }

            if copy_quote_button.clicked() {
                ui.output_mut(|output| output.copied_text = debrid_links.clone());
                self.toasts.add(Toast {
                    text: "Copied debrid links with quotes".into(),
                    kind: ToastKind::Info,
                    options: ToastOptions::default()
                        .duration_in_seconds(5.0)
                        .show_progress(true)
                        .show_icon(true)
                });
            }
        });
        ScrollArea::both()
            .id_source("Debrid Links Area")
            .max_height(ui.available_height())
            .show(ui, |ui| {
                ui.add(
                    TextEdit::multiline(&mut debrid_links)
                        .desired_width(ui.available_width())
                );
            });
    }
}