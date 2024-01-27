use std::{fs, thread};
use std::sync::OnceLock;
use crossbeam_channel::Receiver;
use eframe::egui;
use eframe::egui::{Align2, ComboBox, Context, Id, ScrollArea, TextEdit, Ui, Window};
use eframe::egui::Direction::TopDown;
use egui_toast::{Toast, ToastKind, ToastOptions, Toasts};
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use tokio::runtime::Runtime;
use crate::modules::api::{AllDebridResponse, RealDebridResponse, unlock_links};
use crate::modules::links::{LinkError};

pub enum DebridResponse {
    AllDebrid(Result<AllDebridResponse, LinkError>),
    RealDebrid(Result<RealDebridResponse, LinkError>),
}

#[derive(Default)]
struct Channels {
    pub debrid: Option<Receiver<Vec<DebridResponse>>>
}

#[derive(Clone, Default, PartialEq)]
pub enum DebridService {
    #[default]
    AllDebrid,
    RealDebrid
}

#[derive(Clone, Default, Deserialize)]
struct DebridAPIKeys {
    #[serde(default)]
    all_debrid: String,
    #[serde(default)]
    real_debrid: String
}

#[derive(Default)]
pub struct DebridUI {
    toasts: Toasts,
    channels: Channels,
    debrid_service: DebridService,
    api_key: DebridAPIKeys,
    use_remote_traffic: bool,
    input_links: String,
    input_links_vec: Vec<String>,
    unlocking: bool,
    debrid_links: String,
    error_log_open: bool,
    error_log_text: String,
}

impl DebridUI {
    pub fn display(ctx: &Context, ui: &mut Ui, debrid_ui: &mut DebridUI) {
        debrid_ui.toasts = Toasts::new()
            .anchor(Align2::RIGHT_TOP, (10.0, 10.0))
            .direction(TopDown);

        debrid_ui.display_input_area(ui);
        debrid_ui.display_debrid_links_area(ui);

        debrid_ui.toasts.show(ctx);
    }

    fn display_input_area(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Choose Debrid service:");
            ComboBox::from_id_source("Upload Type")
                .selected_text(match self.debrid_service {
                    DebridService::AllDebrid => "AllDebrid",
                    DebridService::RealDebrid => "RealDebrid"
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.debrid_service, DebridService::AllDebrid, "AllDebrid");
                    ui.selectable_value(&mut self.debrid_service, DebridService::RealDebrid, "RealDebrid");
                });
        });

        ui.horizontal(|ui| {
            ui.label("API Key:");
            match self.debrid_service {
                DebridService::AllDebrid => {
                    ui.add(TextEdit::singleline(&mut self.api_key.all_debrid)
                        .hint_text("Enter your AllDebrid API key here"));
                },
                DebridService::RealDebrid => {
                    ui.add(TextEdit::singleline(&mut self.api_key.real_debrid)
                        .hint_text("Enter your RealDebrid API key here"));
                }
            };

            if ui.button("Read from file").clicked() {
                let api_key_json = fs::read_to_string("./api_key.json");
                match api_key_json {
                    Ok(json_string) => {
                        let api_key_result: Result<DebridAPIKeys, _> = serde_json::from_str(&json_string);
                        match api_key_result {
                            Ok(debrid_api_keys) => {
                                self.api_key = debrid_api_keys;
                                match self.debrid_service {
                                    DebridService::AllDebrid => {
                                        if self.api_key.all_debrid.is_empty() {
                                            self.toasts.add(Toast {
                                                text: "API key not found for AllDebrid".into(),
                                                kind: ToastKind::Warning,
                                                options: ToastOptions::default()
                                                    .duration_in_seconds(5.0)
                                                    .show_progress(true)
                                                    .show_icon(true)
                                            });
                                        }
                                    },
                                    DebridService::RealDebrid => {
                                        if self.api_key.real_debrid.is_empty() {
                                            self.toasts.add(Toast {
                                                text: "API key not found for RealDebrid".into(),
                                                kind: ToastKind::Warning,
                                                options: ToastOptions::default()
                                                    .duration_in_seconds(5.0)
                                                    .show_progress(true)
                                                    .show_icon(true)
                                            });
                                        }
                                    }
                                }

                                self.toasts.add(Toast {
                                    text: "Successfully read API key".into(),
                                    kind: ToastKind::Success,
                                    options: ToastOptions::default()
                                        .duration_in_seconds(5.0)
                                        .show_progress(true)
                                        .show_icon(true)
                                });
                            }
                            Err(_) => {
                                self.toasts.add(Toast {
                                    text: "Failed to parse \"api_key.json\"".into(),
                                    kind: ToastKind::Error,
                                    options: ToastOptions::default()
                                        .duration_in_seconds(5.0)
                                        .show_progress(true)
                                        .show_icon(true)
                                });
                            }
                        }
                    }
                    Err(_) => {
                        self.toasts.add(Toast {
                            text: "Failed to read \"api_key.json\"".into(),
                            kind: ToastKind::Error,
                            options: ToastOptions::default()
                                .duration_in_seconds(5.0)
                                .show_progress(true)
                                .show_icon(true)
                        });
                    },
                }
            }
        });

        ui.heading("Input Links");

        let input_area_height = ui.available_height() / 2.0;
        // ui.set_max_height(input_area_height);
        ScrollArea::both()
            .id_source("Link Input Area")
            .max_height(input_area_height)
            .show(ui, |ui| {
                ui.add(
                    TextEdit::multiline(&mut self.input_links)
                        .hint_text("Paste your links here")
                        .desired_width(ui.available_width()),
                );
            });

        if self.debrid_service == DebridService::RealDebrid {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.use_remote_traffic, "Use remote traffic");

                if ui.label("(?)").hovered() {
                    egui::show_tooltip(ui.ctx(), Id::new("Use Remote Traffic Tooltip"), |ui| {
                        ui.label("Generates RealDebrid links that do not have remote dedicated servers and account sharing protections. \
                    These links can be uploaded directly using remote upload or shared with others.\nImportantly, this uses your \
                    \"remote traffic\" quota, which may incur extra costs.");
                    });
                };
            });
        }

        ui.horizontal(|ui| {
            if ui.button("Unlock links").clicked() {
                self.unlocking = true;
                let (debrid_sender, debrid_receiver) = crossbeam_channel::unbounded();
                self.channels.debrid = Some(debrid_receiver);
                self.input_links_vec = process_links(&self.input_links);
                let links = self.input_links_vec.clone();
                let debrid_service = self.debrid_service.clone();
                let api_key = match self.debrid_service {
                    DebridService::AllDebrid => self.api_key.all_debrid.clone(),
                    DebridService::RealDebrid => self.api_key.real_debrid.clone()
                };
                let use_remote_traffic = self.use_remote_traffic;
                let rt = Runtime::new().unwrap();
                thread::spawn(move || {
                    rt.block_on(async {
                        let client = Client::new();
                        let mut tasks = vec![];
                        for link in links {
                            let link = link.clone();
                            let debrid_service = debrid_service.clone();
                            let api_key = api_key.clone();
                            let client = client.clone();
                            let task = tokio::spawn(async move {
                                unlock_links(&link, debrid_service, &api_key, use_remote_traffic, client).await
                            });
                            tasks.push(task);
                        }
                        let mut debrid_links = vec![];
                        let results = futures::future::join_all(tasks).await;
                        for result in results {
                            debrid_links.push(result.unwrap());
                        }
                        debrid_sender.send(debrid_links).unwrap();
                    });
                });
            }

            if self.unlocking {
                ui.spinner();
                ui.label("Unlocking links...");
            }

            if ui.button("See errors").clicked() {
                self.error_log_open = true;
            }
        });
    }
    pub fn display_error_log(&mut self, ctx: &Context) {
        Window::new("Debrid Error Log")
            .default_width(200.0)
            .open(&mut self.error_log_open)
            .show(ctx, |ui| {
                ScrollArea::vertical()
                    .id_source("Error Log")
                    .min_scrolled_height(ui.available_height())
                    .show(ui, |ui| {
                        let mut error = self.error_log_text.clone();
                        ui.add(TextEdit::multiline(&mut error).desired_width(ui.available_width()));
                    });
            });
    }
    fn display_debrid_links_area(&mut self, ui: &mut Ui) {
        ui.heading("Debrid Links");

        if let Some(receiver) = &self.channels.debrid {
            if let Ok(debrid_results) = receiver.try_recv() {
                let mut links = String::new();
                let mut errors = String::new();
                for (index, response) in debrid_results.iter().enumerate() {
                    match response {
                        DebridResponse::AllDebrid(result) => {
                            match result {
                                Ok(response) => {
                                    links = format!("{}{}\n", links, response.data.link);
                                },
                                Err(error) => {
                                    errors = format!("{}\n\n{} - {:?}", errors, self.input_links_vec[index], error);
                                }
                            }
                        }
                        DebridResponse::RealDebrid(result) => {
                            match result {
                                Ok(response) => {
                                    links = format!("{}{}\n", links, response.link);
                                },
                                Err(error) => {
                                    errors = format!("{}\n\n{} - {:?}", errors, self.input_links_vec[index], error);
                                }
                            }
                        }
                    }
                }
                self.debrid_links = links;
                self.error_log_text = errors;
                self.unlocking = false;
            }
        }

        let mut debrid_links = self.debrid_links.clone().trim().to_string();
        ui.horizontal(|ui| {
            let copy_normal_button = ui.button("Copy");

            if copy_normal_button.clicked() {
                ui.output_mut(|output| output.copied_text = debrid_links.clone());
                self.toasts.add(Toast {
                    text: "Copied Debrid links".into(),
                    kind: ToastKind::Info,
                    options: ToastOptions::default()
                        .duration_in_seconds(5.0)
                        .show_progress(true)
                        .show_icon(true)
                });
            }

            let copy_quote_button = ui.button("Copy as \"{URL}\"");
            if copy_quote_button.hovered() {
                let mut new_debrid_links = String::new();
                let links = debrid_links.split('\n');
                for link in links {
                    new_debrid_links = format!("{}\"{}\"\n", new_debrid_links, link);
                }
                debrid_links = new_debrid_links.trim().to_string()
            }

            if copy_quote_button.clicked() {
                ui.output_mut(|output| output.copied_text = debrid_links.clone());
                self.toasts.add(Toast {
                    text: "Copied Debrid links with quotes".into(),
                    kind: ToastKind::Info,
                    options: ToastOptions::default()
                        .duration_in_seconds(5.0)
                        .show_progress(true)
                        .show_icon(true)
                });
            }

            let copy_quote_and_spaces_button = ui.button("Copy as \"{URL}\" with spaces");
            if copy_quote_and_spaces_button.hovered() {
                let mut new_debrid_links = String::new();
                let links = debrid_links.split('\n');
                for link in links {
                    new_debrid_links = format!("{}\"{}\" ", new_debrid_links, link);
                }
                debrid_links = new_debrid_links.trim().to_string()
            }

            if copy_quote_and_spaces_button.clicked() {
                ui.output_mut(|output| output.copied_text = debrid_links.clone());
                self.toasts.add(Toast {
                    text: "Copied Debrid links with quotes and spaces".into(),
                    kind: ToastKind::Info,
                    options: ToastOptions::default()
                        .duration_in_seconds(5.0)
                        .show_progress(true)
                        .show_icon(true)
                });
            }
        });
        ScrollArea::both()
            .id_source("Debrid Links Area")
            .max_height(ui.available_height())
            .show(ui, |ui| {
                ui.add(
                    TextEdit::multiline(&mut debrid_links)
                        .desired_width(ui.available_width())
                );
            });
    }
}

static LINK_REGEX: OnceLock<Regex> = OnceLock::new();

fn process_links(links: &str) -> Vec<String> {
    let mut detected_links = vec![];
    let link_regex = LINK_REGEX
        .get_or_init(|| Regex::new(r#"(https?://(?:[a-zA-Z]|[0-9]|[$-_@.&+]|[!*\\(),]|%[0-9a-fA-F][0-9a-fA-F]|#)+)"#).unwrap());
    for captures in link_regex.captures_iter(links) {
        let link = captures[0].to_string();
        detected_links.push(link);
    }
    detected_links
}