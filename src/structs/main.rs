use eframe::egui::{self, CentralPanel, menu};

use crate::structs::{download::Download, login::LoginData, settings::Settings, upload::Upload};
use crate::structs::help::Help;

#[derive(Default)]
pub struct Application {
    login: LoginData,
    panel: Panel,
    download: Download,
    upload: Upload,
    show_help: bool,
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
                for (panel, label) in [
                    (Panel::Download, "Download"),
                    (Panel::Upload, "Upload"),
                    (Panel::Settings, "Settings"),
                ] {
                    ui.selectable_value(&mut self.panel, panel, label);
                };
                menu::bar(ui, |ui| {
                    ui.menu_button("Help", |ui| {
                        if ui.button("Show help").clicked() {
                            self.show_help = true;
                            ui.close_menu();
                        };
                    });
                });


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
                Panel::Settings => Settings::show(ctx, ui),
            };

            if self.show_help {
                Help::show(ctx, &mut self.show_help);
            }
        });


    }
}
