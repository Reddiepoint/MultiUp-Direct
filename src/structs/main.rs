
use eframe::egui;
use eframe::egui::{CentralPanel};



use crate::structs::download::Download;
use crate::structs::login::LoginData;
use crate::structs::settings::Settings;
use crate::structs::upload::Upload;

#[derive(Default)]
pub struct Application {
    login: LoginData,
    panel: Panel,
    download: Download,
    upload: Upload,
}

#[derive(Default, PartialEq)]
enum Panel {
    #[default]
    Download,
    Upload,
    Settings,
}

impl Application {}

impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Tabs
        egui::TopBottomPanel::top("Tab Bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (panel, label) in [(Panel::Download, "Download"), (Panel::Upload, "Upload"), (Panel::Settings, "Settings")] {
                    ui.selectable_value(&mut self.panel, panel, label);
                }
            });

        });

        egui::TopBottomPanel::bottom("Statuses").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Logged in as: ".to_string() + &self.login.login);
            });
        });

        CentralPanel::default().show(ctx, |ui| {
            match &self.panel {
                Panel::Download => Download::show(ui, &mut self.download),
                Panel::Upload => Upload::show(ctx, ui),
                Panel::Settings => Settings::show(ctx, ui)
            }
        });
    }
}
