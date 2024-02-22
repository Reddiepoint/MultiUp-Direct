#![windows_subsystem = "windows"]

mod modules;

use std::path::PathBuf;
use eframe::egui::ViewportBuilder;
use crate::modules::app::{DOCUMENTATION, MultiUpDirect, TabBar};
use crate::modules::upload::UploadType;


fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_drag_and_drop(true)
            .with_resizable(true)
            .with_inner_size((1280.0, 800.0)),
        centered: true,
        ..Default::default()
    };

    let mut app = MultiUpDirect::default();
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() > 1 {
        if args[1] == "--help" {
            println!("See {} for help.", DOCUMENTATION);
        } else if args[1] == "--upload" {
            app.tab_bar = TabBar::Upload;
            if args[2] == "disk_upload" {
                app.upload_ui.upload_type = UploadType::Disk;
            }
            for arg in args.into_iter().skip(3) {
                let path = PathBuf::from(arg);
                if path.is_dir() {
                    for file in path.read_dir().unwrap().flatten() {
                        if file.path().is_file() {
                            app.upload_ui.disk_upload_settings.file_paths.push(file.path());
                            app.upload_ui.disk_upload_settings.file_names.push(String::new());
                        }
                    }
                } else if path.is_file() {
                    app.upload_ui.disk_upload_settings.file_paths.push(path);
                    app.upload_ui.disk_upload_settings.file_names.push(String::new());
                }
            }
        }
    }

    eframe::run_native(
        "MultiUp Direct",
        options,
        Box::new(|_cc| Box::<MultiUpDirect>::new(app)),
    )
}
