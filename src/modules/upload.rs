use reqwest::{Client, multipart};
use std::error::Error;
use std::num::ParseIntError;
use std::thread;
use crossbeam_channel::{Receiver, TryRecvError};
use eframe::egui::{Align2, ComboBox, Context, TextBuffer, TextEdit, Ui, Window};
use eframe::egui::Direction::TopDown;
use egui_toast::{Toast, ToastKind, ToastOptions, Toasts};
use tokio::runtime::Runtime;
use crate::modules::api::{get_fastest_server, Login, LoginResponse, MultiUpUploadResponse};
use crate::modules::links::LinkError;

#[derive(Default)]
struct Channels {
    login: Option<Receiver<Result<LoginResponse, LinkError>>>,
}

impl Channels {
    fn new(login_receiver: Option<Receiver<Result<LoginResponse, LinkError>>>) -> Self {
        Self {
            login: login_receiver
        }
    }
}

#[derive(Default, PartialEq)]
pub enum UploadType {
    #[default]
    Remote,
}

#[derive(Default)]
pub struct UploadUI {
    channels: Channels,
    toasts: Toasts,
    display_login: bool,
    login_details: Login,
    login_response: LoginResponse,
    upload_type: UploadType,
}

impl UploadUI {
    pub fn display(ctx: &Context, ui: &mut Ui, upload_ui: &mut UploadUI) {
        upload_ui.toasts = Toasts::new()
            .anchor(Align2::RIGHT_TOP, (10.0, 10.0))
            .direction(TopDown);

        upload_ui.display_login(ui);
        upload_ui.display_login_window(ctx, ui);
        upload_ui.display_upload_types(ui);
        match upload_ui.upload_type {
            UploadType::Remote => upload_ui.display_remote_upload_ui(ui)
        };
        upload_ui.toasts.show(ctx);
    }

    fn display_login_window(&mut self, ctx: &Context, ui: &mut Ui) {
        Window::new("Login").open(&mut self.display_login).show(ctx, |ui| {
            ui.heading("Please log in or enter your user ID");

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Username:");
                        ui.add(TextEdit::singleline(&mut self.login_details.username)
                            .desired_width(ui.available_width() / 2.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Password:");
                        ui.add(TextEdit::singleline(&mut self.login_details.password)
                            .desired_width(ui.available_width() / 2.0));
                    });

                    if ui.button("Login").clicked() {
                        let (login_sender, login_receiver) = crossbeam_channel::unbounded();
                        self.channels = Channels::new(Some(login_receiver));

                        let rt = Runtime::new().unwrap();
                        let login_details = self.login_details.clone();
                        thread::spawn(move || {
                            rt.block_on(async {
                                let login_result = login_details.login().await;

                                let _ = login_sender.send(login_result);
                            });
                        });
                    }
                });

                ui.separator();

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("User ID");
                        ui.text_edit_singleline(&mut self.login_details.user_id);
                    });
                    if ui.button("Login").clicked() {
                        let id = match self.login_details.user_id.parse::<u64>() {
                            Ok(id) => Some(id),
                            Err(_) => {
                                self.toasts.add(Toast {
                                    text: "Invalid ID".into(),
                                    kind: ToastKind::Error,
                                    options: ToastOptions::default()
                                        .duration_in_seconds(5.0)
                                        .show_progress(true)
                                        .show_icon(true)
                                });

                                None
                            }
                        };
                        self.login_response.user = id;
                    }
                });
            });
        });
    }
    fn display_login(&mut self, ui: &mut Ui) {
        if let Some(receiver) = &self.channels.login {
            if let Ok(response) = receiver.try_recv() {
                match response {
                    Ok(login_response) => {
                        match login_response.error.as_str() {
                            "success" => {
                                self.login_response = login_response;
                            }
                            _ => {
                                self.toasts.add(Toast {
                                    text: format!("Failed to log in: {:?}", login_response.error).into(),
                                    kind: ToastKind::Error,
                                    options: ToastOptions::default()
                                        .duration_in_seconds(10.0)
                                        .show_progress(true)
                                        .show_icon(true)
                                });
                            }
                        }
                    },
                    Err(error) => {
                        self.toasts.add(Toast {
                            text: format!("Failed to log in: {:?}", error).into(),
                            kind: ToastKind::Error,
                            options: ToastOptions::default()
                                .duration_in_seconds(5.0)
                                .show_progress(true)
                                .show_icon(true)
                        });
                    }
                }
            }
        }

        let mut user = match &self.login_response.login {
            Some(user) => user.to_string(),
            None => "Anonymous".to_string()
        };

        if let Some(id) = &self.login_response.user {
            user = format!("{} ({})", user, id);
        }
        ui.horizontal(|ui| {
            ui.label(format!("Logged in as: {}", user));
            let login_text = if !self.login_details.user_id.is_empty() || self.login_response.user.is_some() {
                "Change user"
            } else {
                "Login"
            };

            if ui.button(login_text).clicked() {
                self.display_login = true;
            }
        });
    }

    fn display_upload_types(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Choose upload type:");
            ComboBox::from_id_source("Upload Type")
                .selected_text(match self.upload_type {
                    UploadType::Remote => "Remote Upload",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.upload_type, UploadType::Remote, "Remote Upload");
                });
        });
    }

    fn display_remote_upload_ui(&mut self, ui: &mut Ui) {}
}

#[tokio::test]
async fn test_download_and_upload_file_with_reqwest() {
    let url = vec!["https://v2w3x4.debrid.it/dl/2zmoaog89f0/cis-33828253.pdf", "https://v2w3x4.debrid.it/dl/2zmtn7j4919/cis-33828253.pdf"];
    match stream_file(&url).await {
        Ok(_response) => {
            // println!("{}", response.url.unwrap());
        },
        Err(error) => {
            eprintln!("{}", error);
        }
    };
}

async fn stream_file(download_urls: &[&str]) -> Result<(), Box<dyn Error>> {
    let api_url = get_fastest_server().await?;

    // Create a reqwest client
    let client = Client::new();

    // Download the file
    let mut responses = vec![];
    for download_url in download_urls {
        let download_response = client.get(download_url.as_str()).send().await?;
        responses.push(download_response);
    }

    let mut files = vec![];
    for download_response in responses {
        let content_disposition = download_response.headers().get(reqwest::header::CONTENT_DISPOSITION);
        let file_name = content_disposition
            .and_then(|cd| cd.to_str().ok())
            .and_then(|cd| cd.split(';').find(|&s| s.trim_start().starts_with("filename=")))
            .and_then(|filename_param| filename_param.split('=').nth(1))
            .map(|name| name.trim_matches('"').to_string());


        let content_length = download_response.headers().get(reqwest::header::CONTENT_LENGTH)
            .and_then(|cl| cl.to_str().ok())
            .and_then(|cl| cl.parse::<u64>().ok());

        // Stream the file directly without saving to disk, converting it to a compatible stream
        let file_stream = download_response.bytes_stream();

        // Convert the stream into a Body for the multipart form
        let file_body = reqwest::Body::wrap_stream(file_stream);

        // Create a multipart/form-data object with the stream
        let part = multipart::Part::stream_with_length(file_body, content_length.unwrap_or(0))
            .file_name(file_name.unwrap_or("file_name".to_string()));

        files.push(part);
    }

    // Create a multipart/form-data object
    let mut form = multipart::Form::new()
        // .part("files", part)
        .text("project-hash", "testing");

    for part in files {
        form = form.part("files[]", part);
    }

    // Upload the file
    let response = client.post(api_url)
        .multipart(form)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await?;
        eprintln!("Error uploading file: {}", error_text);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Upload failed")));
    }

    // Output the response body for the upload
    let upload_response = response.json::<MultiUpUploadResponse>().await?;
    match upload_response.files.is_empty() {
        true => {
            eprintln!("No files in the upload response");
        }
        false => {
            println!("Upload Response: {:?}", upload_response);
        }
    }

    Ok(())
}
