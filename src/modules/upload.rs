use std::collections::HashSet;
use reqwest::{Client, multipart};
use std::thread;
use crossbeam_channel::Receiver;
use eframe::egui;
use eframe::egui::{Align2, Button, Checkbox, ComboBox, Context, Id, ScrollArea, TextEdit, Ui, Window};
use eframe::egui::Direction::TopDown;
use egui_toast::{Toast, ToastKind, ToastOptions, Toasts};
use tokio::runtime::Runtime;
use crate::modules::api::{AddProject, AvailableHosts, get_fastest_server, Login, LoginResponse, MultiUpUploadResponse, UploadedFileDetails};
use crate::modules::links::LinkError;

#[derive(Default)]
struct Channels {
    login: Option<Receiver<Result<LoginResponse, LinkError>>>,
    hosts: Option<Receiver<Result<AvailableHosts, LinkError>>>,
    upload: Option<Receiver<Result<MultiUpUploadResponse, LinkError>>>,
}

// impl Channels {
//     fn new(login_receiver: Option<Receiver<Result<LoginResponse, LinkError>>>,
//            host_receiver: Option<Receiver<Result<AvailableHosts, LinkError>>>,
//            upload_receiver: Option<Receiver<Result<MultiUpUploadResponse, LinkError>>>
//     ) -> Self {
//         Self {
//             login: login_receiver,
//             hosts: host_receiver,
//             upload: upload_receiver
//         }
//     }
// }

#[derive(Default, PartialEq)]
pub enum UploadType {
    Disk,
    #[default]
    Remote,
}

#[derive(Clone)]
pub struct RemoteUploadSettings {
    is_project: bool,
    project_name: String,
    project_password: String,
    project_description: String,
    upload_links: String,
    file_names: String,
    data_streaming: bool,
    hosts: HashSet<String>
}

impl Default for RemoteUploadSettings {
    fn default() -> Self {
        Self {
            is_project: false,
            project_name: String::new(),
            project_password: String::new(),
            project_description: String::new(),
            upload_links: String::new(),
            file_names: String::new(),
            data_streaming: true,
            hosts: HashSet::new()
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
    hosts: AvailableHosts,
    uploading: bool,
    multiup_links: Vec<String>,
}

impl UploadUI {
    pub fn display(ctx: &Context, ui: &mut Ui, upload_ui: &mut UploadUI) {
        upload_ui.toasts = Toasts::new()
            .anchor(Align2::RIGHT_TOP, (10.0, 10.0))
            .direction(TopDown);

        upload_ui.display_login(ui);
        upload_ui.display_login_window(ctx);
        upload_ui.display_upload_types(ui);
        match upload_ui.upload_type {
            UploadType::Disk => todo!(),
            UploadType::Remote => upload_ui.display_remote_upload_ui(ui),
        };
        upload_ui.toasts.show(ctx);
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
            let login_text = if self.login_response.user.is_some() {
                "Change user"
            } else {
                "Login"
            };

            if ui.button(login_text).clicked() {
                self.show_login_window = true;
            }
        });
    }

    fn display_login_window(&mut self, ctx: &Context) {
        Window::new("Login").open(&mut self.show_login_window).show(ctx, |ui| {
            ui.heading("Please log into your MultiUp account");

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
                        self.channels.login = Some(login_receiver);
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
            });
        });
    }
    fn display_upload_types(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Choose upload type:");
            ComboBox::from_id_source("Upload Type")
                .selected_text(match self.upload_type {
                    UploadType::Disk => "Disk Upload",
                    UploadType::Remote => "Remote Upload",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.upload_type, UploadType::Disk, "Disk Upload");
                    ui.selectable_value(&mut self.upload_type, UploadType::Remote, "Remote Upload");
                });
        });
    }

    fn display_remote_upload_ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.remote_upload_settings.is_project, "Upload as project");
            ui.vertical(|ui| {
                if self.remote_upload_settings.is_project {
                    ui.add(TextEdit::singleline(&mut self.remote_upload_settings.project_name)
                        .hint_text("Enter project name"));
                    ui.add(TextEdit::singleline(&mut self.remote_upload_settings.project_password)
                        .hint_text("Enter project password (optional)"));
                    ui.add(TextEdit::singleline(&mut self.remote_upload_settings.project_description)
                        .hint_text("Enter project description (optional"));
                }
            });
        });

        ui.horizontal(|ui| {
            ui.add_enabled(false, Checkbox::new(&mut self.remote_upload_settings.data_streaming, "Enable data streaming"));
            if ui.label("(?)").hovered() {
                egui::show_tooltip(ui.ctx(), Id::new("Data Streaming Tooltip"), |ui| {
                    ui.label("Data streaming allows for better remote upload support by download and uploading the file at the same time. \
                This bypasses remote upload restrictions (e.g. AllDebrid) because the connection is created by a regular computer. \
                This comes at the cost of bandwidth usage (for downloading and uploading), compared to the regular remote upload (theoretically negligible), \
                but the data is not saved to disk.");
                });
            };
        });


        ScrollArea::vertical().id_source("Remote Upload Links")
            .max_height(ui.available_height() / 2.0)
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    let half_width = ui.available_width() / 2.0;

                    ui.add(TextEdit::multiline(&mut self.remote_upload_settings.upload_links)
                        .hint_text("Enter the URLS you want to remotely upload, separated by a newline.")
                        .desired_width(half_width));

                    ui.add(TextEdit::multiline(&mut self.remote_upload_settings.file_names)
                        .hint_text("Enter custom file names (optional). The name should match the same line as order as the URLs. \
                        Leave a newline to use the default names.")
                        .desired_width(half_width));
                });
            });

        if self.hosts.hosts.is_empty() && self.channels.hosts.is_none() {
            self.toasts.add(Toast {
                text: "Getting hosts".into(),
                kind: ToastKind::Warning,
                options: ToastOptions::default()
                    .duration_in_seconds(5.0)
                    .show_progress(true)
                    .show_icon(true)
            });

            let (hosts_sender, hosts_receiver) = crossbeam_channel::unbounded();
            self.channels.hosts = Some(hosts_receiver);
            let rt = Runtime::new().unwrap();
            thread::spawn(move || {
                rt.block_on(async {
                    let hosts = AvailableHosts::get().await;

                    hosts_sender.send(hosts).unwrap();
                });
            });
        }

        if let Some(hosts) = &self.channels.hosts {
            if let Ok(hosts) = hosts.try_recv() {
                match hosts {
                    Ok(mut hosts) => {
                        for (_host, details) in hosts.hosts.iter_mut() {
                            if details.selection == "true" {
                                details.selected = true;
                            } else if details.selection == "false" {
                                details.selected = false;
                            }
                        }
                        self.hosts = hosts;
                    }
                    Err(error) => {
                        self.toasts.add(Toast {
                            text: format!("Failed to get hosts: {:?}", error).into(),
                            kind: ToastKind::Error,
                            options: ToastOptions::default()
                                .duration_in_seconds(10.0)
                                .show_progress(true)
                                .show_icon(true)
                        });
                    }
                }
            }
        }

        ui.heading("Hosts");

        ui.horizontal(|ui| {
            if ui.button("Select all").clicked() {
                for (_host, details) in self.hosts.hosts.iter_mut() {
                    details.selected = true;
                }
            }

            if ui.button("Select default").clicked() {
                for (host, details) in self.hosts.hosts.iter_mut() {
                    details.selected = self.hosts.default.contains(host);
                }
            }

            let select_user_hosts = ui.button("Select user favourites");
            if select_user_hosts.hovered() {
                egui::show_tooltip(ui.ctx(), Id::new("User Favourites Tooltip"), |ui| {
                    ui.label("Deselect all hosts to use the account's favourite hosts.");
                });
            }

            if select_user_hosts.clicked() {
                for (_host, details) in self.hosts.hosts.iter_mut() {
                    details.selected = false;
                }
            }
        });

        ui.columns(5, |columns| {
            for (host, details) in self.hosts.hosts.iter_mut() {
                columns[0].checkbox(&mut details.selected, format!("{} ({} GB)", host, details.size / 1024));
                columns.rotate_left(1);
            }
        });

        ui.horizontal(|ui| {
            if ui.add_enabled(!self.uploading, Button::new("Upload to MultiUp")).clicked() {
                self.uploading = true;
                let (upload_sender, upload_receiver) = crossbeam_channel::unbounded();
                self.channels.upload = Some(upload_receiver);
                self.remote_upload_settings.hosts = self.hosts.hosts.iter()
                    .filter(|(_, details)| details.selected)
                    .map(|(host, _)| host.to_string())
                    .collect();
                let remote_upload_settings = self.remote_upload_settings.clone();
                let login_response = self.login_response.clone();
                let rt = Runtime::new().unwrap();
                thread::spawn(move || {
                    rt.block_on(async {
                        let (urls, file_names) = process_urls_and_names(&remote_upload_settings.upload_links, &remote_upload_settings.file_names);
                        let password = if !remote_upload_settings.project_password.is_empty() {
                            Some(remote_upload_settings.project_password)
                        } else {
                            None
                        };
                        let description = if !remote_upload_settings.project_description.is_empty() {
                            Some(remote_upload_settings.project_description)
                        } else {
                            None
                        };
                        let user = login_response.user.map(|user| user.to_string());

                        let project_hash = if remote_upload_settings.is_project {
                            let project = AddProject::new(
                                remote_upload_settings.project_name,
                                password,
                                description,
                                user
                            );
                            match project.add_project().await {
                                Ok(response) => response.hash,
                                Err(error) => {
                                    upload_sender.send(Err(error)).unwrap();
                                    return;
                                }
                            }
                        } else {
                            None
                        };
                        let response = stream_file(&urls, &file_names, login_response, remote_upload_settings.hosts, project_hash.clone()).await;
                        if let Ok(mut response) = response {
                            response.project_hash = project_hash;
                            upload_sender.send(Ok(response)).unwrap();
                        } else {
                            upload_sender.send(response).unwrap();
                        }
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
                        }],
                        project_hash: None
                    });
                    let mut multiup_links = vec![];
                    for file in response.files {
                        if let Some(url) = file.url {
                            multiup_links.push(url);
                        }
                    }

                    if let Some(hash) = response.project_hash {
                        multiup_links.push(format!("https://multiup.io/en/project/{}", hash));
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
    match stream_file(&urls, &file_names, LoginResponse::default(), HashSet::new(), None).await {
        Ok(_response) => {
            // println!("{}", response.url.unwrap());
        },
        Err(error) => {
            eprintln!("{:?}", error);
        }
    };
}

async fn stream_file(download_urls: &[String], file_names: &[String], login_response: LoginResponse, hosts: HashSet<String>, project_hash: Option<String>) -> Result<MultiUpUploadResponse, LinkError> {
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
    let mut form = multipart::Form::new();

    if let Some(id) = login_response.user {
        form = form.text("user", id.to_string());
    }

    if let Some(hash) = project_hash {
        form = form.text("project-hash", hash);
    }

    for host in hosts {
        form = form.text(host, "true");
    }

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
    // println!("Upload Response: {:?}", upload_response);
    Ok(upload_response)
}
