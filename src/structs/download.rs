use std::sync::mpsc;

use eframe::egui::{ScrollArea, TextEdit, Ui};
use pollster::FutureExt;
use tokio::runtime::Runtime;

use crate::functions::download::generate_direct_links;
use crate::functions::filter::filter_links;
use crate::structs::filter::FilterMenu;
use crate::structs::hosts::Link;

pub struct Download {
    pub mirror_links_input: String,
    pub check_status: bool,
    pub direct_links: Vec<Link>,
    pub links_to_display: String,
    pub filter: FilterMenu,
}

impl Default for Download {
    fn default() -> Self {
        Download {
            mirror_links_input: String::new(),
            check_status: true,
            direct_links: vec![],
            links_to_display: String::new(),
            filter: FilterMenu::default(),
        }
    }
}

impl Download {
    fn mirror_links_box(&mut self, ui: &mut Ui) {
        let height = ui.available_height() / 2.0;
        ui.label("Enter your MultiUp links:");
        ui.vertical(|ui| {
            ui.set_max_height(height);
            ScrollArea::vertical().min_scrolled_height(ui.available_height()).show(ui, |ui| {
                ui.add(TextEdit::multiline(&mut self.mirror_links_input)
                    .hint_text("Enter your MultiUp links separated by a new line\nSupports short and long links")
                    .desired_width(ui.available_width())
                );
            });
        });
    }

    fn generate_links_ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Check status
            ui.checkbox(
                &mut self.check_status,
                "Check host status",
            );
            // Generate links
            if ui.button("Generate links").clicked() {
                // Create runtime
                let rt = Runtime::new().expect("Unable to create runtime");
                let _ = rt.enter();
                let (tx, rx) = mpsc::sync_channel(1);
                let mirror_links = self.mirror_links_input.clone();
                let check_status = self.check_status;
                std::thread::spawn(move || {
                    let generated_links = rt.block_on(async {
                        generate_direct_links(&mirror_links, check_status).await
                    });
                    tx.send(generated_links)
                });

                let (generated_links, links_hosts) = rx.recv().unwrap();
                self.direct_links = generated_links;
                self.filter.hosts = links_hosts;
            };
        });
    }

    fn display_direct_links(&mut self, ui: &mut Ui) {
        let height = ui.available_height();
        self.links_to_display = filter_links(&self.direct_links, &self.filter);
        // Generated direct links output
        ui.horizontal(|ui| {
            ui.set_height(height);
            ui.vertical(|ui| {
                ui.label("Direct links: ");
                ScrollArea::vertical()
                    .min_scrolled_height(ui.available_height())
                    .show(ui, |ui| {
                        let output = ui.add(
                            TextEdit::multiline(&mut self.links_to_display)
                                .desired_width(ui.available_width() - 200.0),
                        );
                        output.context_menu(|ui| {
                            if ui.button("Copy links").clicked() {
                                ui.output_mut(|output| output.copied_text = self.links_to_display.clone());
                                ui.close_menu();
                            }
                        });
                    });
            });

            FilterMenu::show(ui, &mut self.filter);
        });
    }


    pub fn show(ui: &mut Ui, download: &mut Download) {
        // Enter download links box
        download.mirror_links_box(ui);
        // Check hosts and generate button
        download.generate_links_ui(ui);
        // Output and filter menu
        download.display_direct_links(ui);
    }
}
