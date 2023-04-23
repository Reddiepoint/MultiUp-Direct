use eframe::egui::Ui;
use crate::structs::login::{Login, LoginData};

pub async fn login(login: Login) -> Result<LoginData, String> {
    let client = reqwest::Client::new();
    match client.post("https://multiup.org/api/login").form(&login).send().await.unwrap().json::<LoginData>().await {
        Ok(login_data) => Ok(login_data),
        Err(error) => Err("Error: ".to_string() + &error.to_string())
    }
}