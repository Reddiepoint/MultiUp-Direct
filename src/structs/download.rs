use eframe::egui::{CentralPanel, Context};
use serde_json::to_string;

#[derive(Default)]
pub struct Download {
    multiup_links: String,
    download_links: Vec<String>
}

impl Download {
    pub fn show(ctx: &Context, download: &mut Download) {
        CentralPanel::default().show(ctx, |ui| {
            // Enter download links box
            ui.label("Enter your MultiUp links:");
            ui.text_edit_multiline(&mut download.multiup_links);

            // Scrape button
            if ui.button("Generate download links").clicked() {
                download.download_links = download.multiup_links
                    .split('\n')
                    .map(|link| link.trim().replace("download", "en/mirror"))
                    .collect::<Vec<String>>();

                for link in &download.download_links {

                }
            };




        });
    }
}