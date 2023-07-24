use eframe::egui::{self, menu, CentralPanel};

use crate::modules::{
    download::Download,
    help::{Help, VERSION},
};



#[derive(Default)]
pub struct Application {
    //login: LoginData,
    panel: Panel,
    download: Download,
    //upload: Upload,
    help: Help,
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
                {
                    let (panel, label) = (Panel::Download, "Download");
                    //(Panel::Upload, "Upload"),
                    //(Panel::Settings, "Settings"),
                    ui.selectable_value(&mut self.panel, panel, label);
                }
                menu::bar(ui, |ui| {
                    ui.menu_button("Help", |ui| {
                        if ui.button("Show help").clicked() {
                            self.help.show_help = true;
                            ui.close_menu();
                        };

                        ui.separator();

                        if ui.button("Check for updates").clicked() {
                            let (tx, rx) = crossbeam_channel::unbounded();
                            //(self.help.update_sender, self.help.update_receiver) = (Some(tx), Some(rx));
                            self.help.new_channels(tx, rx);
                            self.help.show_update = true;
                            ui.close_menu();
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

            if self.help.show_help {
                Help::show_help(ctx, &mut self.help.show_help);
            }

            if self.help.show_update {
                Help::show_update(ctx, &mut self.help);
            }
        });
    }
}
