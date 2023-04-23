mod functions;
mod structs;

use crate::structs::login::{LoginData, Login};
use eframe::egui;
use functions::upload::*;
use reqwest::Error;
use crate::structs::main::Application;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        ..Default::default()
    };
    eframe::run_native("My MultiUp Client", options, Box::new(|_cc| Box::<Application>::default()))

    
}
