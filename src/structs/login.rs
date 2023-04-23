use crate::functions::login::login;
use eframe::egui::{self, Context};
use pollster::FutureExt;
use serde::{Deserialize, Serialize};
use std::mem;
use egui::widgets::text_edit::TextEdit;
use tokio::runtime::Runtime;

#[derive(Default, Serialize, Clone)]
pub struct Login {
    username: String,
    password: String,
    #[serde(skip_serializing)]
    error: String
}

impl Login {
    pub fn login(ctx: &Context, login_sign: &mut Login) -> (bool, LoginData) {
        let mut login_data = LoginData::default();
        let mut show = true;

        egui::Window::new("Login").default_width(500.0).resizable(true).show(ctx, |ui| {
            ui.set_width(ui.available_width());
            egui::Grid::new("").show(ui, |ui| {
                // Username
                ui.label("Username: ");
                ui.add(TextEdit::singleline(&mut login_sign.username).desired_width(ui.available_width()));
                ui.end_row();

                // Password
                ui.label("Password: ");
                ui.add(TextEdit::singleline(&mut login_sign.password).password(true).desired_width(500.0));
                ui.end_row();


                let rt = Runtime::new().expect("Unable to create new runtime");
                let _rt_enter = rt.enter();
                // Log in button
                if ui.button("Login").clicked() {
                    // let login_response = async { login(mem::take(login_sign)).await };
                    // let login_response = login_response.block_on();
                    let (tx, rx) = std::sync::mpsc::sync_channel(0);
                    let login_clone = login_sign.clone();
                    std::thread::spawn(move || {
                        let data = rt.block_on(async { login(login_clone).await});
                        tx.send(data)
                    });
                    
                    match rx.recv().unwrap() {
                        Ok(login) => {
                            if login.error.as_str() == "success" {
                                show = false;
                                login_data = login;
                            } else {
                                login_sign.error = "Error: ".to_string() + &login.error
                            };
                        }
                        Err(_error) => {
                            login_sign.error = "Error: Invalid username or password".to_string()
                        }
                    };
                };
                ui.label(&login_sign.error);
            });
        });
        (show, login_data)
    }
}

#[derive(Default, Debug, Deserialize)]
pub struct LoginData {
    pub error: String,
    pub login: String,
    pub user: u64,
    pub account_type: String,
    pub premium_days_left: String,
}
