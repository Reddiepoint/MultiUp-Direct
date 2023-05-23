use eframe::egui::{self, menu, CentralPanel};

use crate::constants::help::VERSION;
use crate::structs::help::Help;
use crate::structs::{download::Download};

#[derive(Default)]
pub struct Application {
    //login: LoginData,
    panel: Panel,
    download: Download,
    //upload: Upload,
    show_help: bool,
}

#[derive(Default, PartialEq)]
enum Panel {
    #[default]
    Download,
    //Upload,
    //Settings,
}

impl Application {}

impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Tabs
        egui::TopBottomPanel::top("Tab Bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (panel, label) in [
                    (Panel::Download, "Download"),
                    //(Panel::Upload, "Upload"),
                    //(Panel::Settings, "Settings"),
                ] {
                    ui.selectable_value(&mut self.panel, panel, label);
                }
                menu::bar(ui, |ui| {
                    ui.menu_button("Help", |ui| {
                        if ui.button("Show help").clicked() {
                            self.show_help = true;
                            ui.close_menu();
                        };

                        if ui.button("Changelog").clicked() {

                        }

                        ui.separator();
                        if ui.button("Check for updates").clicked() {

                        }

                        ui.label(format!("Version {}", VERSION))
                    });
                });
            });
        });

        //egui::TopBottomPanel::bottom("Statuses").show(ctx, |ui| {
        //    ui.horizontal(|ui| {
        //        ui.label("Logged in as: ".to_string() + &self.login.login);
        //    });
        //});

        CentralPanel::default().show(ctx, |ui| {
            match &self.panel {
                Panel::Download => Download::show(ui, &mut self.download),
                //Panel::Upload => Upload::show(ctx, ui),
                //Panel::Settings => Settings::show(ui),
            };

            if self.show_help {
                Help::show_help(ctx, &mut self.show_help);
            }
        });
    }
}
