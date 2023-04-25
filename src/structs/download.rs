use std::sync::mpsc;
use eframe::egui::{CentralPanel, Checkbox, Context, ScrollArea, TextEdit, Ui};
use pollster::FutureExt;
use reqwest::Error;
use serde_json::to_string;
use tokio::runtime::Runtime;
use crate::functions::download::{fix_mirror_links, generate_direct_links};
use crate::functions::filter::filter_links;
use crate::functions::hosts::check_validity;
use crate::structs::filter::FilterLinksCriteria;
use crate::structs::hosts::Link;


pub struct Download {
    pub mirror_links: String,
    pub check_status: bool,
    pub direct_download_links: Vec<Link>,
    pub display_download_links: String,
    pub filter: FilterLinksCriteria
}

impl Default for Download {
    fn default() -> Self {
        Download {
            mirror_links: String::new(),
            check_status: true,
            direct_download_links: vec![],
            display_download_links: String::new(),
            filter: FilterLinksCriteria::default()
        }
    }
}

impl Download {
    pub fn enter_mirror_links(&mut self, ui: &mut Ui, ) {
        ui.label("Enter your MultiUp links:");
        let height = ui.available_height() / 2.0;
        ui.vertical(|ui| {
            ui.set_max_height(height);
            ScrollArea::vertical().min_scrolled_height(ui.available_height()).show(ui, |ui| {
                ui.add(TextEdit::multiline(&mut self.mirror_links)
                    .hint_text("Enter your MultiUp links separated by a new line\nSupports short and long links")
                    .desired_width(ui.available_width())
                );
            });
        });
    }

    pub fn show(ui: &mut Ui, download: &mut Download) {
        // Enter download links box
        download.enter_mirror_links(ui);

        // Scrape button
        ui.horizontal(|ui| {
            // Check status
            ui.checkbox(&mut download.check_status, "Check host status (will take a longer amount of time)");
            // Generate links
            if ui.button("Generate download links").clicked() {
                // Create runtime
                let rt = Runtime::new().expect("Unable to create runtime");
                let _ = rt.enter();
                let (tx, rx) = mpsc::sync_channel(0);
                let mirror_links = download.mirror_links.clone();
                let check_status = download.check_status.clone();
                let host_filter = download.filter.hosts.clone();
                std::thread::spawn(move || {
                    let generated_links = rt.block_on(async {
                        generate_direct_links(&mirror_links, &check_status, &host_filter).await
                    });
                    tx.send(generated_links)
                });

                let (generated_links, links_hosts) = rx.recv().unwrap();
                download.direct_download_links = generated_links;
                download.filter.hosts = links_hosts;
            };
        });

        let height = ui.available_height();
        // Generated direct links output
        ui.horizontal(|ui| {
            ui.set_height(height);
            ui.vertical(|ui| {
                ui.label("Direct download links: ");
                ScrollArea::vertical().min_scrolled_height(ui.available_height()).show(ui, |ui| {
                    ui.add(TextEdit::multiline({
                        download.display_download_links = filter_links(&download.direct_download_links, &download.filter);
                        &mut download.display_download_links
                    })
                        .desired_width(ui.available_width() - 200.0));
                });
            });

            FilterLinksCriteria::show(ui, &mut download.filter);
        });
    }
}

