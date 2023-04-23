use crate::structs::{
    login::{Login, LoginData},
    server::FastestServer,
};
use eframe::egui;
use eframe::egui::{CentralPanel, TextEdit};
use tokio::runtime::Runtime;
use crate::functions::download::get_link_hosts;
use crate::functions::upload::{get_upload_url, upload};
use crate::structs::download::Download;
use crate::structs::upload::Upload;

#[derive(Default)]
pub struct Application {
    show_login: bool,
    login: Login, // Username:Password
    login_data: LoginData,
    upload_url: FastestServer,
    scraper_url: String,
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
        egui::TopBottomPanel::top("my_tab_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.panel, Panel::Download, "Download");
                ui.selectable_value(&mut self.panel, Panel::Upload, "Upload");
                ui.selectable_value(&mut self.panel, Panel::Settings, "Settings");
            });
        });

        CentralPanel::default().show(ctx, |ui| {
            match &self.panel {
                Panel::Download => {
                    Download::show(ctx, &mut self.download)
                }
                Panel::Upload => {}
                Panel::Settings => {}
            }
        });
        // Main window
        // egui::CentralPanel::default().show(ctx, |ui| {
        //     ui.label({
        //         if self.login_data.login.is_empty() {
        //             "Not logged in"
        //         } else {
        //             &self.login_data.login
        //         }
        //     });
        //     if ui.button("Log In").clicked() {
        //         self.show_login = true;
        //     }
        //     if ui.button("Test").clicked() {
        //         let rt = Runtime::new().expect("Unable to create new runtime");
        //         let _rt_enter = rt.enter();
        //         let (tx, rx) = std::sync::mpsc::sync_channel(0);
        //
        //         std::thread::spawn(move || {
        //             let data = rt.block_on(async {
        //                 let fastest_server = get_upload_url().await.unwrap();
        //
        //                 upload(fastest_server).await
        //             });
        //             tx.send(data)
        //         });
        //
        //         let url = rx.recv().unwrap();
        //         println!("{}", url);
        //     }
        //
        //     ui.add(TextEdit::singleline(&mut self.scraper_url));
        //     if ui.button("Scrape").clicked() {
        //         let rt = Runtime::new().expect("Unable to create new runtime");
        //         let _rt_enter = rt.enter();
        //         let (tx, rx) = std::sync::mpsc::sync_channel(0);
        //         let url = self.scraper_url.clone();
        //         std::thread::spawn(move || {
        //             let data = rt.block_on(async {
        //                 get_file_hosts(&url).await
        //             });
        //             tx.send(data)
        //         });
        //         let url = rx.recv().unwrap().unwrap();
        //         println!("{}", url);
        //     };
        //
        //     ui.horizontal(|ui| {
        //         ui.selectable_value(&mut self.panel, Panel::Download, "One");
        //         ui.selectable_value(&mut self.panel, Panel::Upload, "Two");
        //     });
        //
        //
        // });
        //
        // let mut login_data = LoginData::default();
        // if self.show_login {
        //     (self.show_login, login_data) =
        //         Login::login(ctx, &mut self.login);
        // }
    }
}
