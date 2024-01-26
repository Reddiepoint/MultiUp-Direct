use std::{fs, thread};
use std::sync::OnceLock;
use crossbeam_channel::Receiver;
use eframe::egui::{Align2, Context, ScrollArea, TextEdit, Ui, Window};
use eframe::egui::Direction::TopDown;
use egui_toast::{Toast, ToastKind, ToastOptions, Toasts};
use regex::Regex;
use reqwest::Client;
use tokio::runtime::Runtime;
use crate::modules::api::{AllDebridResponse, unlock_links};
use crate::modules::links::{LinkError};


#[derive(Default)]
struct Channels {
    pub debrid: Option<Receiver<Vec<Result<AllDebridResponse, LinkError>>>>
}

#[derive(Default)]
enum DebridService {
    #[default]
    AllDebrid,
    RealDebrid
}

#[derive(Default)]
pub struct DebridUI {
    toasts: Toasts,
    channels: Channels,
    debrid_service: DebridService,
    api_key: String,
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
            ui.label("API Key:");
            ui.add(TextEdit::singleline(&mut self.api_key)
                .hint_text("Enter your AllDebrid API key here"));

            if ui.button("Read from file").clicked() {
                let api_key = fs::read_to_string("./api_key.txt");
                match api_key {
                    Ok(key) => {
                        self.api_key = key.trim().to_string();
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
                            text: "Failed to read \"api_key.txt\"".into(),
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


        ui.horizontal(|ui| {
            if ui.button("Unlock links").clicked() {
                self.unlocking = true;
                let (debrid_sender, debrid_receiver) = crossbeam_channel::unbounded();
                self.channels.debrid = Some(debrid_receiver);
                self.input_links_vec = process_links(&self.input_links);
                let links = self.input_links_vec.clone();
                let api_key = self.api_key.clone();
                let rt = Runtime::new().unwrap();
                thread::spawn(move || {
                    rt.block_on(async {
                        let client = Client::new();
                        let mut debrid_links = vec![];
                        for link in links {
                            let debrid_link = unlock_links(&link, &api_key, client.clone()).await;
                            debrid_links.push(debrid_link);
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
                for (index, link) in debrid_results.iter().enumerate() {
                    match link {
                        Ok(response) => {
                            links = format!("{}{}\n", links, response.data.link);
                        },
                        Err(error) => {
                            errors = format!("{}\n\n{} - {:?}", errors, self.input_links_vec[index], error);
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
                    text: "Copied debrid links".into(),
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
                    text: "Copied debrid links with quotes".into(),
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
                    text: "Copied debrid links with quotes and spaces".into(),
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