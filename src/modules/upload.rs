use reqwest::{Client, multipart, Response, Url};
use std::error::Error;
use std::num::ParseIntError;
use std::thread;
use crossbeam_channel::{Receiver, TryRecvError};
use eframe::egui::{Align2, Button, Checkbox, ComboBox, Context, ScrollArea, TextBuffer, TextEdit, Ui, Window};
use eframe::egui::Direction::TopDown;
use egui_toast::{Toast, ToastKind, ToastOptions, Toasts};
use tokio::runtime::Runtime;
use crate::modules::api::{get_fastest_server, Login, LoginResponse, MultiUpUploadResponse, UploadedFileDetails};
use crate::modules::links::LinkError;

#[derive(Default)]
struct Channels {
    login: Option<Receiver<Result<LoginResponse, LinkError>>>,
    upload: Option<Receiver<Result<MultiUpUploadResponse, LinkError>>>,
}

impl Channels {
    fn new(login_receiver: Option<Receiver<Result<LoginResponse, LinkError>>>, upload_receiver: Option<Receiver<Result<MultiUpUploadResponse, LinkError>>>, ) -> Self {
        Self {
            login: login_receiver,
            upload: upload_receiver,
        }
    }
}

#[derive(Default, PartialEq)]
pub enum UploadType {
    #[default]
    Remote,
}

#[derive(Clone)]
pub struct RemoteUploadSettings {
    is_project: bool,
    project_name: String,
    upload_links: String,
    file_names: String,
    data_streaming: bool
}

impl Default for RemoteUploadSettings {
    fn default() -> Self {
        Self {
            is_project: false,
            project_name: String::new(),
            upload_links: String::new(),
            file_names: String::new(),
            data_streaming: true
        }
    }
}

#[derive(Default)]
pub struct UploadUI {
    channels: Channels,
    toasts: Toasts,
    show_login_window: bool,
    login_details: Login,
    login_response: LoginResponse,
    upload_type: UploadType,
    remote_upload_settings: RemoteUploadSettings,
    uploading: bool,
    multiup_links: Vec<String>,
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
        let mut open = self.show_login_window;
        Window::new("Login").open(&mut self.show_login_window).show(ctx, |ui| {
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
                            .password(true)
                            .desired_width(ui.available_width() / 2.0));
                    });

                    if ui.button("Login").clicked() {
                        let (login_sender, login_receiver) = crossbeam_channel::unbounded();
                        self.channels = Channels::new(Some(login_receiver), self.channels.upload.clone());

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
                        let id = match self.login_details.user_id.trim().parse::<u64>() {
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
                        // self.show_login_window = false;
                        open = false;
                    }
                });
            });
        });
        if !open {
            self.show_login_window = false;
        }
    }
    fn display_login(&mut self, ui: &mut Ui) {
        if let Some(receiver) = &self.channels.login {
            if let Ok(response) = receiver.try_recv() {
                match response {
                    Ok(login_response) => {
                        match login_response.error.as_str() {
                            "success" => {
                                self.toasts.add(Toast {
                                    text: format!("Logged in as {:?}", login_response.login.as_ref().unwrap_or(&"user".to_string())).into(),
                                    kind: ToastKind::Success,
                                    options: ToastOptions::default()
                                        .duration_in_seconds(5.0)
                                        .show_progress(true)
                                        .show_icon(true)
                                });
                                self.login_response = login_response;
                                self.show_login_window = false;
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
            None => {
                if let Some(id) = self.login_response.user {
                    id.to_string()
                } else {
                    "Anonymous".to_string()
                }
            }
        };

        if let Some(id) = &self.login_response.user {
            user = format!("{} (ID: {})", user, id);
        }
        ui.horizontal(|ui| {
            ui.label(format!("Logged in as: {}", user));
            let login_text = if !self.login_details.user_id.is_empty() || self.login_response.user.is_some() {
                "Change user"
            } else {
                "Login"
            };

            if ui.button(login_text).clicked() {
                self.show_login_window = true;
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

    fn display_remote_upload_ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.add_enabled(self.login_response.user.is_some(),
                           Checkbox::new(&mut self.remote_upload_settings.is_project, "Upload as project")
            );
            if self.remote_upload_settings.is_project {
                ui.add(TextEdit::singleline(&mut self.remote_upload_settings.project_name)
                    .hint_text("Enter project name"));
            }
        });

        ui.add_enabled(false, Checkbox::new(&mut self.remote_upload_settings.data_streaming, "Enable data streaming"));

        ScrollArea::vertical().id_source("Remote Upload Links")
            .max_height(ui.available_height() / 2.0)
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    let half_width = ui.available_width() / 2.0;

                    ui.add(TextEdit::multiline(&mut self.remote_upload_settings.upload_links)
                        .hint_text("Enter the URLS you want to remotely upload.")
                        .desired_width(half_width));

                    ui.add(TextEdit::multiline(&mut self.remote_upload_settings.file_names)
                        .hint_text("Enter custom file names (optional). The name should match the same line as order as the URLs. \
                        Leave a newline to use the default names.")
                        .desired_width(half_width));
                });
            });

        ui.horizontal(|ui| {
            if ui.add_enabled(!self.uploading, Button::new("Upload to MultiUp")).clicked() {
                self.uploading = true;
                let (upload_sender, upload_receiver) = crossbeam_channel::unbounded();
                self.channels = Channels::new(self.channels.login.clone(), Some(upload_receiver));
                let rt = Runtime::new().unwrap();
                let remote_upload_settings = self.remote_upload_settings.clone();
                let login_response = self.login_response.clone();

                thread::spawn(move || {
                    rt.block_on(async {
                        let (urls, file_names) = process_urls_and_names(&remote_upload_settings.upload_links, &remote_upload_settings.file_names);
                        let response = stream_file(login_response, &urls, &file_names).await;
                        upload_sender.send(response).unwrap();
                    });
                });
            }

            if self.uploading {
                ui.spinner();
                ui.label("Uploading...");
            }
        });

        ui.heading("MultiUp Links");
        ScrollArea::vertical().id_source("Uploaded MultiUp Links").show(ui, |ui| {
            // ui.add(TextEdit::multiline(&mut self.multiup_links))
            if let Some(response) = &self.channels.upload {
                if let Ok(result) = response.try_recv() {
                    self.uploading = false;
                    let response = result.unwrap_or_else(|error| MultiUpUploadResponse {
                        files: vec![UploadedFileDetails {
                            name: None,
                            hash: None,
                            size: None,
                            file_type: None,
                            url: Some(format!("{:?}", error)),
                            sid: None,
                            user: None,
                            delete_url: None,
                            delete_type: None
                        }]
                    });
                    let mut multiup_links = vec![];
                    for file in response.files {
                        if let Some(url) = file.url {
                            multiup_links.push(url);
                        }
                    }

                    self.multiup_links = multiup_links;
                }
            }
            let mut links = self.multiup_links.join("\n");
            ui.add(TextEdit::multiline(&mut links).desired_width(ui.available_width()));
        });
    }
}

fn process_urls_and_names(urls: &str, names: &str) -> (Vec<String>, Vec<String>) {
    let urls = urls.split('\n').map(|x| x.trim().to_string()).collect::<Vec<String>>();
    let names = names.split('\n').map(|x| x.trim().to_string()).collect::<Vec<String>>();
    (urls, names)
}

#[tokio::test]
async fn test_download_and_upload_file_with_reqwest() {
    let urls = vec!["https://v2w3x4.debrid.it/dl/2zmoaog89f0/cis-33828253.pdf".to_string(), "https://v2w3x4.debrid.it/dl/2zmtn7j4919/cis-33828253.pdf".to_string()];
    let file_names = vec!["".to_string(), "".to_string()];
    match stream_file(LoginResponse::default(), &urls, &file_names).await {
        Ok(_response) => {
            // println!("{}", response.url.unwrap());
        },
        Err(error) => {
            eprintln!("{:?}", error);
        }
    };
}

async fn stream_file(login_response: LoginResponse, download_urls: &[String], file_names: &[String]) -> Result<MultiUpUploadResponse, LinkError> {
    let api_url = get_fastest_server().await?;

    // Create a reqwest client
    let client = Client::new();

    // Download the file
    let mut responses = vec![];
    for download_url in download_urls {
        let download_response = match client.get(download_url).send().await {
            Ok(response) => response,
            Err(error) => return Err(LinkError::Reqwest(error))
        };

        responses.push(download_response);
    }

    let mut files = vec![];
    for (index, download_response) in responses.into_iter().enumerate() {
        let content_disposition = download_response.headers().get(reqwest::header::CONTENT_DISPOSITION);
        println!("First: {:?}", content_disposition);
        println!("Header: {:?}", download_response.headers());
        let file_name = match file_names.get(index) {
            Some(name) => {
                if name.is_empty() {
                    content_disposition
                        .and_then(|cd| cd.to_str().ok())
                        .and_then(|cd| cd.split(';').find(|&s| s.trim_start().starts_with("filename=")))
                        .and_then(|filename_param| filename_param.split('=').nth(1))
                        .map(|name| name.trim_matches('"').to_string())
                } else {
                    Some(name.to_string())
                }
            }
            None => {
                content_disposition
                    .and_then(|cd| cd.to_str().ok())
                    .and_then(|cd| cd.split(';').find(|&s| s.trim_start().starts_with("filename=")))
                    .and_then(|filename_param| filename_param.split('=').nth(1))
                    .map(|name| name.trim_matches('"').to_string())
            }
        };

        let file_name = match file_name {
            Some(name) => name,
            None => {
                download_urls[index].split('/').last().unwrap().to_string()
            }
        };

        println!("End: {:?}", file_name);



        let content_length = download_response.headers().get(reqwest::header::CONTENT_LENGTH)
            .and_then(|cl| cl.to_str().ok())
            .and_then(|cl| cl.parse::<u64>().ok());

        // Stream the file directly without saving to disk, converting it to a compatible stream
        let file_stream = download_response.bytes_stream();

        // Convert the stream into a Body for the multipart form
        let file_body = reqwest::Body::wrap_stream(file_stream);

        // Create a multipart/form-data object with the stream
        let part = multipart::Part::stream_with_length(file_body, content_length.unwrap_or(0))
            .file_name(file_name);

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
    let response = match client.post(api_url).multipart(form).send().await {
        Ok(response) => response,
        Err(error) => return Err(LinkError::APIError(error.to_string()))
    };

    // let status = response.status();
    // if !status.is_success() {
    //     let error_text = response.text().await;
    //     eprintln!("Error uploading file: {}", error_text);
    //     return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Upload failed")));
    // }

    // Output the response body for the upload
    let upload_response = match response.json::<MultiUpUploadResponse>().await {
        Ok(response) => response,
        Err(error) => return Err(LinkError::APIError(error.to_string()))
    };
    // match upload_response.files.is_empty() {
    //     true => {
    //         eprintln!("No files in the upload response");
    //     }
    //     false => {
    //         println!("Upload Response: {:?}", upload_response);
    //     }
    // }

    Ok(upload_response)
}
