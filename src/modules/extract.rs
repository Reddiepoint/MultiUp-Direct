use std::collections::{BTreeSet, HashSet};
use async_recursion::async_recursion;
use eframe::egui::{Align2, Button, CollapsingHeader, Context, Label, ScrollArea, Sense, TextEdit, TopBottomPanel, Ui, Window};
use egui_extras::{Column, TableBuilder};
use regex::Regex;
use reqwest::Client;
use scraper::{ElementRef, Selector};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::Instant;
use crossbeam_channel::{Receiver, Sender};
use eframe::egui::Direction::TopDown;
use egui_toast::{Toast, ToastKind, ToastOptions, Toasts};
use tokio::runtime::Runtime;
use tokio::sync::Semaphore;
use crate::modules::api::{MultiUpLinkInformation, recheck_validity_api};
use crate::modules::filter::FilterMenu;
use crate::modules::general::get_page_html;
use crate::modules::links::{DirectLink, DownloadLink, LinkError, MultiUpLink, ProjectLink};


#[derive(Default)]
struct Channels {
    direct_links: Option<Receiver<Vec<MultiUpLink>>>,
    cancel: Option<Sender<bool>>,
}

impl Channels {
    fn new(direct_links_receiver: Option<Receiver<Vec<MultiUpLink>>>, cancel_sender: Option<Sender<bool>>) -> Self {
        Self {
            direct_links: direct_links_receiver,
            cancel: cancel_sender
        }
    }
}

#[derive(Default)]
pub struct ExtractUI {
    multiup_links: String,
    recheck_validity: bool,
    currently_extracting: bool,
    completed_links: Vec<MultiUpLink>,
    toasts: Toasts,
    shown_toast: bool,
    channels: Channels,
    error_log_open: bool,
    error_log_text: String,
    search_filter: String,
    filter: FilterMenu,
    selection: (Option<usize>, Option<usize>),
    visible_links: Vec<String>,
    selected_links: HashSet<String>,
    direct_links: HashSet<String>
}

impl ExtractUI {
    pub fn display(ctx: &Context, ui: &mut Ui, extract_ui: &mut ExtractUI) {
        extract_ui.display_input_area(ui);
        extract_ui.display_footer(ctx);
        extract_ui.display_output_area(ui);
        extract_ui.toasts.show(ctx);
    }

    fn display_input_area(&mut self, ui: &mut Ui) {
        ui.heading("MultiUp Links");

        let input_area_height = ui.available_height() / 4.0;
        ui.set_max_height(input_area_height);
        ScrollArea::both()
            .id_source("MultiUp Link Input Area")
            .max_height(input_area_height)
            .show(ui, |ui| {
                ui.add(
                    TextEdit::multiline(&mut self.multiup_links)
                        .hint_text("Paste your MultiUp links here")
                        .desired_width(ui.available_width()),
                );
            });


        // UI elements related to the extraction of links
        ui.horizontal(|ui| {
            // Recheck validity checkbox
            ui.checkbox(&mut self.recheck_validity, "Recheck link validity");
            // Extract links button

            if ui
                .add_enabled(!self.currently_extracting, Button::new("Extract direct links"))
                .clicked()
            {
                self.currently_extracting = true;


                let (direct_links_sender, direct_links_receiver) = crossbeam_channel::unbounded();
                let (cancel_sender, cancel_receiver) = crossbeam_channel::unbounded();
                self.channels = Channels::new(Some(direct_links_receiver), Some(cancel_sender));

                // Main extraction function
                let rt = Runtime::new().unwrap();
                let multiup_links = self.multiup_links.clone();
                let recheck_validity = self.recheck_validity;
                thread::spawn(move || {
                    rt.block_on(async {
                        let multiup_links = extract_direct_links(&multiup_links, recheck_validity, cancel_receiver).await;
                        let _ = direct_links_sender.send(multiup_links);
                    });
                });
            }

            // Generation text and cancel extraction button
            if self.currently_extracting {
                ui.spinner();
                ui.label("Extracting links...");
                if ui.button("Cancel now").clicked() {
                    if let Some(sender) = &self.channels.cancel {
                        let _ = sender.send(true);
                        self.channels.cancel.take();
                    }
                }
            }

            if let Some(receiver) = &self.channels.direct_links {
                if let Ok(multiup_links) = receiver.try_recv() {
                    self.completed_links = multiup_links;
                    self.currently_extracting = false;
                    self.shown_toast = false;
                    self.filter.update_hosts(&self.completed_links);
                }
            }

            if !self.completed_links.is_empty() {
                let mut total_links: u32 = 0;
                let mut successful_links: u32 = 0;
                for link in self.completed_links.iter() {
                    match link {
                        MultiUpLink::Project(project) => {
                            match &project.status {
                                Some(Ok(_)) => {
                                    for link in project.download_links.as_ref().unwrap() {
                                        if link.status.as_ref().is_some_and(|status| status.is_ok()) {
                                            total_links += 1;
                                            successful_links += 1;
                                        } else {
                                            total_links += 1;
                                        }
                                    }
                                },
                                _ => {
                                    total_links += 1;
                                }
                            }
                        }
                        MultiUpLink::Download(download) => {
                            match &download.status {
                                Some(Ok(_)) => {
                                    total_links += 1;
                                    successful_links += 1
                                }
                                _ => {
                                    total_links += 1;
                                }
                            }
                        }
                    }
                }

                ui.label(format!("{}/{} extracted successfully", successful_links, total_links));

                self.toasts = Toasts::new()
                    .anchor(Align2::RIGHT_TOP, (10.0, 10.0))
                    .direction(TopDown);

                if successful_links != total_links && !self.shown_toast {
                    self.toasts.add(Toast {
                        text: "Error extracting".into(),
                        kind: ToastKind::Error,
                        options: ToastOptions::default()
                            .duration_in_seconds(5.0)
                            .show_progress(true)
                            .show_icon(true)
                    });

                    self.shown_toast = true;
                }
            }


            if ui.button("See errors").clicked() {
                let mut errors = String::new();
                for link in self.completed_links.iter() {
                    match link {
                        MultiUpLink::Project(project) => {
                            match &project.status {
                                Some(status) => {
                                    match status {
                                        Ok(_) => {
                                            for link in project.download_links.as_ref().unwrap() {
                                                if link.status.as_ref().is_none() {
                                                    errors = format!("{}\n\n{} - {}", errors, &link.original_link, "Unknown");
                                                } else if let Err(error) = link.status.as_ref().unwrap() {
                                                    errors = format!("{}\n\n{} - {:?}", errors, &link.original_link, error);
                                                }
                                            }
                                        }
                                        Err(error) => {
                                            errors = format!("{}\n\n{} - {:?}", errors, &project.original_link, error);
                                        }
                                    }
                                }
                                None => {
                                    errors = format!("{}\n\n{} - {}", errors, &project.original_link, "Unknown");
                                }
                            }
                        }
                        MultiUpLink::Download(download) => {
                            match &download.status {
                                Some(status) => {
                                    match status {
                                        Ok(_) => {},
                                        Err(error) => {
                                            errors = format!("{}\n\n{} - {:?}", errors, &download.original_link, error);
                                        }
                                    }
                                },
                                None => {
                                    errors = format!("{}\n\n{} - {}", errors, &download.original_link, "Unknown");
                                }
                            }
                        }
                    }
                }
                self.error_log_text = errors;
                self.error_log_open = true;
            }
        });
    }

    pub fn display_error_log(&mut self, ctx: &Context) {
        Window::new("Extract Error Log")
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

    fn display_output_area(&mut self, ui: &mut Ui) {
        ui.heading("Extracted Links");

        ui.horizontal(|ui| {
            ui.add(TextEdit::singleline(&mut self.search_filter)
                .id_source("Link Search Filter")
                .hint_text("Type to search for files"));

            if ui.button("X").clicked() {
                self.search_filter = String::new();
            }
        });

        let height = ui.available_height();
        self.visible_links = vec![];
        self.direct_links = HashSet::new();
        let mut url_labels = vec![];
        ui.horizontal(|ui| {
            ui.set_height(height);
            let output_box_width = 0.80 * ui.available_width();
            ScrollArea::vertical()
                .id_source("Direct Links Output")
                .max_height(ui.available_height() - 20.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        for link in &self.completed_links {
                            match link {
                                MultiUpLink::Project(project) => {
                                    if let Some(Ok(())) = project.status {
                                        for link in project.download_links.as_ref().unwrap() {
                                            let filtered_links = self.filter.filter_links(link);
                                            for link in filtered_links {
                                                self.direct_links.insert(link);
                                            }
                                        }
                                        CollapsingHeader::new(&project.name)
                                            .id_source(&project.link_id)
                                            .default_open(true)
                                            .show(ui, |ui| {
                                                TableBuilder::new(ui)
                                                    .column(Column::exact(output_box_width))
                                                    .vscroll(false)
                                                    .body(|body| {
                                                        let heights = calculate_row_heights(project.download_links.as_ref().unwrap(), &self.filter);
                                                        let download_links: Vec<&DownloadLink> = project.download_links.as_ref().unwrap().iter().collect();

                                                        body.heterogeneous_rows(heights.into_iter(), |mut row| {
                                                            let row_index = row.index();
                                                            let link = download_links[row_index];
                                                            let mut searched = false;
                                                            if let Some(information) = &link.link_information {
                                                                let mut display_information = String::new();
                                                                if let Some(file_name) = &information.file_name {
                                                                    display_information += file_name;
                                                                    searched = file_name.to_lowercase().contains(&self.search_filter.to_lowercase());
                                                                }
                                                                if let Some(description) = &information.description {
                                                                    display_information += format!(" - {}", description).as_str();
                                                                }
                                                                if let Some(file_size) = &information.size {
                                                                    display_information += format!(" ({} bytes)", file_size).as_str();
                                                                }
                                                                if let Some(date_upload) = &information.date_upload {
                                                                    display_information += format!(" | Uploaded on {}", date_upload).as_str();
                                                                }
                                                                if searched {
                                                                    row.col(|ui| {
                                                                        CollapsingHeader::new(&display_information).id_source(&link.link_id).default_open(true).show(ui, |ui| {
                                                                            let filtered_links = self.filter.filter_links(link);
                                                                            for link in &filtered_links {
                                                                                self.visible_links.push(link.clone());
                                                                                let url_label = ui.add(Label::new(link).sense(Sense::click()));
                                                                                if url_label.hovered() || self.selected_links.contains(link) {
                                                                                    url_label.clone().highlight();
                                                                                }
                                                                                if url_label.clicked() {
                                                                                    let (control_is_down, shift_is_down) = ui.ctx().input(|ui| (ui.modifiers.ctrl, ui.modifiers.shift));
                                                                                    if control_is_down {
                                                                                        if !self.selected_links.remove(link.as_str()) {
                                                                                            self.selected_links.insert(link.clone());
                                                                                        };
                                                                                        self.selection = (None, None);
                                                                                    } else if shift_is_down {
                                                                                        if self.selection.0.is_none() {
                                                                                            self.selection.0 = Some(self.visible_links.iter().position(|url| url == link).unwrap());
                                                                                        } else {
                                                                                            self.selection.1 = Some(self.visible_links.iter().position(|url| url == link).unwrap());
                                                                                        };
                                                                                    } else {
                                                                                        self.selection.0 = Some(self.visible_links.iter().position(|url| url == link).unwrap());
                                                                                    }
                                                                                }

                                                                                url_labels.push((url_label, link.clone()));
                                                                            }
                                                                        });
                                                                    });
                                                                }
                                                            }
                                                        });
                                                    });
                                            });
                                    }
                                },
                                MultiUpLink::Download(download) => {
                                    if let Some(Ok(())) = download.status {
                                        let information = download.link_information.as_ref().unwrap();
                                        let mut display_information = String::new();
                                        let mut searched = false;
                                        if let Some(file_name) = &information.file_name {
                                            display_information += file_name;
                                            searched = file_name.to_lowercase().contains(&self.search_filter.to_lowercase());
                                        }
                                        if let Some(description) = &information.description {
                                            display_information += format!(" - {}", description).as_str();
                                        }
                                        if let Some(file_size) = &information.size {
                                            display_information += format!(" ({} bytes)", file_size).as_str();
                                        }
                                        if let Some(date_upload) = &information.date_upload {
                                            display_information += format!(" | Uploaded on {}", date_upload).as_str();
                                        }

                                        let filtered_links = self.filter.filter_links(download);
                                        let number_of_direct_links = filtered_links.len() as f32;
                                        let height = 20.0 + number_of_direct_links * 20.0;
                                        if searched {
                                            ui.push_id(&download.link_id, |ui| {
                                                TableBuilder::new(ui).column(Column::exact(output_box_width)).body(|body| {
                                                    body.rows(height, 1, |mut row| {
                                                        // let row_index = row.index();
                                                        row.col(|ui| {
                                                            CollapsingHeader::new(&display_information).id_source(&download.link_id).default_open(true).show(ui, |ui| {
                                                                for link in &filtered_links {
                                                                    self.visible_links.push(link.clone());
                                                                    let url_label = ui.add(Label::new(link).sense(Sense::click()));
                                                                    if url_label.hovered() || self.selected_links.contains(link) {
                                                                        url_label.clone().highlight();
                                                                    }
                                                                    if url_label.clicked() {
                                                                        let (control_is_down, shift_is_down) = ui.ctx().input(|ui| (ui.modifiers.ctrl, ui.modifiers.shift));
                                                                        if control_is_down {
                                                                            if !self.selected_links.remove(link.as_str()) {
                                                                                self.selected_links.insert(link.clone());
                                                                            };
                                                                            self.selection = (None, None);
                                                                        } else if shift_is_down {
                                                                            if self.selection.0.is_none() {
                                                                                self.selection.0 = Some(self.visible_links.iter().position(|url| url == link).unwrap());
                                                                            } else {
                                                                                self.selection.1 = Some(self.visible_links.iter().position(|url| url == link).unwrap());
                                                                            };
                                                                        } else {
                                                                            self.selection.0 = Some(self.visible_links.iter().position(|url| url == link).unwrap());
                                                                        }
                                                                    }

                                                                    url_labels.push((url_label, link.clone()))
                                                                }
                                                            });
                                                        });
                                                    });
                                                });
                                            });
                                            for link in filtered_links {
                                                self.direct_links.insert(link);
                                            }
                                        }
                                    }
                                }
                            };
                        }
                    });
                });

            self.filter.show(ui, &self.completed_links);
        });

        if self.selection.1.is_some() && self.selection.0 > self.selection.1 {
            (self.selection.0, self.selection.1) = (self.selection.1, self.selection.0)
        }

        if let (Some(index_1), Some(index_2)) = self.selection {
            self.visible_links[index_1..=index_2].iter().for_each(|url| { self.selected_links.insert(url.clone()); });
            if ui.ctx().input(|ui| !ui.modifiers.shift) {
                self.selection = (None, None);
            }
        }

        if self.search_filter.is_empty() {
            let selected_links = self.selected_links.clone();
            for link in selected_links.iter() {
                if !self.direct_links.contains(link) {
                    self.selected_links.remove(link);
                }
            }
        }


        for (url_label, link) in url_labels {
            url_label.context_menu(|ui| {
                if ui.button("Copy link").clicked() {
                    ui.output_mut(|output| output.copied_text = link.to_string());
                    self.toasts.add(Toast {
                        text: "Copied link".into(),
                        kind: ToastKind::Info,
                        options: ToastOptions::default()
                            .duration_in_seconds(5.0)
                            .show_progress(true)
                            .show_icon(true)
                    });
                    ui.close_menu();
                };

                if !self.selected_links.is_empty() && ui.button("Copy selected links").clicked() {
                    let selected_links: Vec<String> = self.selected_links.iter().cloned().collect();

                    ui.output_mut(|output| output.copied_text = selected_links.join("\n"));
                    self.toasts.add(Toast {
                        text: "Copied selected links".into(),
                        kind: ToastKind::Info,
                        options: ToastOptions::default()
                            .duration_in_seconds(5.0)
                            .show_progress(true)
                            .show_icon(true)
                    });
                    ui.close_menu();
                };

                if ui.button("Copy all links").clicked() {
                    let urls = self.direct_links.iter().cloned().collect::<Vec<String>>();
                    ui.output_mut(|output| output.copied_text = urls.join("\n"));
                    self.toasts.add(Toast {
                        text: "Copied all links".into(),
                        kind: ToastKind::Info,
                        options: ToastOptions::default()
                            .duration_in_seconds(5.0)
                            .show_progress(true)
                            .show_icon(true)
                    });
                    ui.close_menu();
                };

                ui.separator();

                if ui.button("Open link in browser").clicked() {
                    match webbrowser::open(&link) {
                        Ok(_) => {
                            self.toasts.add(Toast {
                                text: "Opened link".into(),
                                kind: ToastKind::Success,
                                options: ToastOptions::default()
                                    .duration_in_seconds(5.0)
                                    .show_progress(true)
                                    .show_icon(true)
                            });
                        }
                        Err(error) => {
                            self.toasts.add(Toast {
                                text: format!("Failed to open link: {}", error).into(),
                                kind: ToastKind::Info,
                                options: ToastOptions::default()
                                    .duration_in_seconds(5.0)
                                    .show_progress(true)
                                    .show_icon(true)
                            });
                        }
                    };
                    ui.close_menu();
                };

                if !self.selected_links.is_empty() && ui.button("Open selected links in browser").clicked() {
                    let mut success = String::new();
                    for link in self.selected_links.iter() {
                        if let Err(error) = webbrowser::open(link) {
                            success = error.to_string()
                        };
                    }

                    match success.is_empty() {
                        true => {
                            self.toasts.add(Toast {
                                text: "Opened links".into(),
                                kind: ToastKind::Success,
                                options: ToastOptions::default()
                                    .duration_in_seconds(5.0)
                                    .show_progress(true)
                                    .show_icon(true)
                            });
                        }
                        false => {
                            self.toasts.add(Toast {
                                text: format!("Failed to open links: {}", success).into(),
                                kind: ToastKind::Info,
                                options: ToastOptions::default()
                                    .duration_in_seconds(5.0)
                                    .show_progress(true)
                                    .show_icon(true)
                            });
                        }
                    };

                    ui.close_menu();
                };

                if ui.button("Open all links in browser").clicked() {
                    let mut success = String::new();
                    for link in self.direct_links.iter() {
                        if let Err(error) = webbrowser::open(link) {
                            success = error.to_string()
                        };
                    }

                    match success.is_empty() {
                        true => {
                            self.toasts.add(Toast {
                                text: "Opened all links".into(),
                                kind: ToastKind::Success,
                                options: ToastOptions::default()
                                    .duration_in_seconds(5.0)
                                    .show_progress(true)
                                    .show_icon(true)
                            });
                        }
                        false => {
                            self.toasts.add(Toast {
                                text: format!("Failed to open all links: {}", success).into(),
                                kind: ToastKind::Info,
                                options: ToastOptions::default()
                                    .duration_in_seconds(5.0)
                                    .show_progress(true)
                                    .show_icon(true)
                            });
                        }
                    };
                    ui.close_menu();
                };

                ui.separator();

                if ui.button("Select all links").clicked() {
                    for link in self.direct_links.iter() {
                        self.selected_links.insert(link.clone());
                    }
                    ui.close_menu();
                }

                if !self.selected_links.is_empty() && ui.button("Deselect all links").clicked() {
                    self.selected_links = HashSet::new();
                    self.toasts.add(Toast {
                        text: "Deselected links".into(),
                        kind: ToastKind::Info,
                        options: ToastOptions::default()
                            .duration_in_seconds(5.0)
                            .show_progress(true)
                            .show_icon(true)
                    });
                    ui.close_menu();
                }
            });
        }
    }

    fn display_footer(&mut self, ctx: &Context) {
        TopBottomPanel::bottom("Information").show(ctx, |ui| {
            ui.label(format!("Selected {}/{} links", self.selected_links.len(), self.direct_links.len()));
        });
    }
}

// Extraction Functions
async fn extract_direct_links(input_text: &str, recheck_validity: bool, cancel_receiver: Receiver<bool>) -> Vec<MultiUpLink> {
    // Detect links
    let detected_links = detect_links(input_text);

    // Process links
    let processed_links = process_links(detected_links, cancel_receiver.clone()).await;

    // Return vec of completed links
    let time_now = Instant::now();
    let completed_links = get_direct_links(processed_links, recheck_validity, cancel_receiver).await;
    let time_taken = time_now.elapsed();
    println!("{}", time_taken.as_secs_f32());
    completed_links
}

/// Detects MultiUp links in the given input text.
fn detect_links(input_text: &str) -> Vec<String> {
    // Create regexes
    let (multiup_regex, _, _, _) = create_regexes();
    // Pre-allocate memory for a vec which contains all detected MultiUp links
    let mut detected_links: Vec<String> = Vec::with_capacity(input_text.lines().count());

    // Detection
    for captures in multiup_regex.captures_iter(input_text) {
        let link = captures[0].to_string();
        detected_links.push(link);
    }

    // Return detected links
    detected_links
}

async fn process_links(detected_links: Vec<String>, cancel_receiver: Receiver<bool>) -> Vec<MultiUpLink> {
    // Create regexes
    let (_, download_regex, mirror_regex, project_regex) = create_regexes();

    // Pre-allocate memory for a vec which contains all detected MultiUp links
    // Follows the system of Vec<(original_link, id, name, is_project, status)>
    let mut processed_links: Vec<MultiUpLink> = Vec::with_capacity(detected_links.len());

    // Store tasks for processing project links, which will be awaited after other links are processed
    let mut project_processing_tasks = Vec::new();

    // Processing
    for link in detected_links {
        let cancel_receiver = cancel_receiver.clone();
        if project_regex.is_match(&link) {
            let link = link.clone();
            let processing_task = tokio::spawn(async move {
                process_project_link(&link, cancel_receiver).await
            });
            project_processing_tasks.push(processing_task);
        } else if mirror_regex.is_match(&link) {
            let download_link = MultiUpLink::Download(process_non_project_link(&link.clone(), &mirror_regex));
            if !processed_links.contains(&download_link) {
                processed_links.push(download_link);
            }
        } else if download_regex.is_match(&link) {
            let download_link = MultiUpLink::Download(process_non_project_link(&link.clone(), &download_regex));
            if !processed_links.contains(&download_link) {
                processed_links.push(download_link);
            }
        }
    }

    let project_links = futures::future::join_all(project_processing_tasks).await;
    for link in project_links {
        processed_links.append(&mut vec![link.unwrap()])
    }

    processed_links
}

static DOWNLOAD_REGEX: OnceLock<Regex> = OnceLock::new();
static MIRROR_REGEX: OnceLock<Regex> = OnceLock::new();
static PROJECT_REGEX: OnceLock<Regex> = OnceLock::new();
static MULTIUP_REGEX: OnceLock<Regex> = OnceLock::new();

/// Creates and initialises regular expressions used for matching different types of links.
///
/// Returns a tuple containing four regular expressions:
/// - `multiup_regex`: Matches all MultiUp links.
/// - `download_regex`: Matches download links.
/// - `mirror_regex`: Matches mirror links.
/// - `project_regex`: Matches project links.
pub fn create_regexes() -> (Regex, Regex, Regex, Regex) {
    // All MultiUp links
    let multiup_regex = MULTIUP_REGEX
        .get_or_init(|| Regex::new(r"(https?://(www\.)?multiup\.(org|io)/\S+)").unwrap());

    // Download links
    let download_regex = DOWNLOAD_REGEX.get_or_init(|| {
        Regex::new(r"https?://(www\.)?multiup\.(org|io)/(en/)?(download/)?").unwrap()
    });

    // Mirror links
    let mirror_regex = MIRROR_REGEX.get_or_init(|| {
        Regex::new(r"https?://multiup\.(org|io)/en/mirror/").unwrap()
    });

    // Project links
    let project_regex = PROJECT_REGEX.get_or_init(|| {
        Regex::new(r"^https://(www\.)?multiup\.(org|io)/(en/)?project/.*$").unwrap()
    });

    (
        multiup_regex.to_owned(),
        download_regex.to_owned(),
        mirror_regex.to_owned(),
        project_regex.to_owned(),
    )
}

/// Processes a given project link.
///
/// This function takes in a project link, mirror regex, and download regex as inputs,
/// and returns a Project MultiUpLink.
async fn process_project_link(project_link: &str, cancel_receiver: Receiver<bool>) -> MultiUpLink {
    // Download links
    let download_regex = DOWNLOAD_REGEX.get().unwrap();

    // Mirror links
    let mirror_regex = MIRROR_REGEX.get().unwrap();

    let (id, name, download_links) = get_project_information(project_link, cancel_receiver).await;
    let download_links = match download_links {
        Ok(download_links) => download_links,
        Err(error) => {
            let mut project_link = ProjectLink::new(project_link.to_string(), id, name);
            project_link.status = Some(Err(error));
            return MultiUpLink::Project(project_link);
        }
    };
    // let download_links = get_project_download_links(project_link).await?;
    let mut processed_links: HashSet<DownloadLink> = HashSet::with_capacity(download_links.len());

    for link in download_links {
        if mirror_regex.is_match(&link) {
            let download_link = process_non_project_link(&link.clone(), mirror_regex);
            processed_links.insert(download_link);
        } else if download_regex.is_match(&link) {
            let download_link = process_non_project_link(&link.clone(), download_regex);
            processed_links.insert(download_link);
        }
    }

    let mut project_link = ProjectLink::new(project_link.to_string(), id, name);
    project_link.download_links = Some(processed_links);
    project_link.status = Some(Ok(()));
    MultiUpLink::Project(project_link)
}

static PROJECT_DOWNLOAD_LINKS_SELECTOR: OnceLock<Selector> = OnceLock::new();
static PROJECT_TITLE_SELECTOR: OnceLock<Selector> = OnceLock::new();

/// Retrieves information about a project given a project link.
///
/// Parses the project link for an ID, parses the page title for a name and extracts download links.
/// If there is no name, it is set to the ID.
#[async_recursion]
async fn get_project_information(project_link: &str, cancel_receiver: Receiver<bool>) -> (String, String, Result<Vec<String>, LinkError>) {
    let link_parts: Vec<&str> = project_link.split('/').collect();
    let id = link_parts.last().unwrap().to_string();
    let name = id.clone();

    let client = Client::new();
    let html = match get_page_html(project_link, &client, Some(cancel_receiver), 0).await {
        Ok(html) => html,
        Err(error) => {
            return (id, name, Err(error));
        }
    };

    let parsed_page = scraper::Html::parse_document(&html);

    let project_title_selector = PROJECT_TITLE_SELECTOR
        .get_or_init(|| Selector::parse(r#".text-truncate"#).unwrap());
    let name = match parsed_page.select(project_title_selector).next() {
        Some(title) => {
            let title_text = title.text().last().unwrap().to_string();
            match get_project_name_from_title(&title_text) {
                Some(name) => name.to_string(),
                None => id.clone()
            }
        },
        None => id.clone()
    };

    let project_download_links_selector = PROJECT_DOWNLOAD_LINKS_SELECTOR
        .get_or_init(|| Selector::parse(r#"#textarea-links-long"#).unwrap());
    let links = match parsed_page.select(project_download_links_selector).next() {
        Some(links) => {
            Ok(links.inner_html().to_string().split('\n').map(|link| link.to_string()).collect())
        },
        None => return (id, name, Err(LinkError::NoLinks)),
    };
    (id, name, links)
}

/// Extracts the project name from a given title text.
fn get_project_name_from_title(title_text: &str) -> Option<&str> {
    let prefixes = [" / Projet ", " / Project "];
    for prefix in prefixes.iter() {
        if let Some(index) = title_text.find(prefix) {
            let name_start = index + prefix.len();
            let name_end = title_text.find(" (").unwrap_or(title_text.len());
            let name = &title_text[name_start..name_end];
            return Some(name);
        }
    }
    None
}

/// Process a non-project link and return a `DownloadLink` object.
fn process_non_project_link(link: &str, regex: &Regex) -> DownloadLink {
    let link_parts = regex.replace(link, "");
    let mut link_parts = link_parts.split('/');
    let id = link_parts.next().unwrap().to_string();

    DownloadLink::new(link.to_string(), id)
}


async fn get_direct_links(multiup_links: Vec<MultiUpLink>, recheck_validity: bool, cancel_receiver: Receiver<bool>) -> Vec<MultiUpLink> {
    // At the beginning of the function
    let semaphore = Arc::new(Semaphore::new(200));
    let mut tasks = Vec::new();
    let client = Client::new();
    for link in multiup_links {
        let cancel_receiver = cancel_receiver.clone();
        let client = client.clone();
        match link {
            MultiUpLink::Project(project_link) => {
                // Create a task for each project link
                let semaphore = Arc::clone(&semaphore);
                let task = tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    let project = get_direct_links_from_project(project_link, recheck_validity, cancel_receiver, client).await;
                    MultiUpLink::Project(project)
                });
                tasks.push(task);
            }
            MultiUpLink::Download(download_link) => {
                // Create a task for each download link
                let semaphore = Arc::clone(&semaphore);
                let task = tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    let download = get_direct_links_from_download_link(download_link, recheck_validity, cancel_receiver, client).await;
                    MultiUpLink::Download(download)
                });
                tasks.push(task);
            }
        }
    }

    let mut multiup_links = vec![];
    // Wait for all tasks to complete
    let results = futures::future::join_all(tasks).await;
    for link in results {
        multiup_links.push(link.unwrap());
    }

    multiup_links
}

async fn get_direct_links_from_project(mut project_link: ProjectLink, recheck_validity: bool, cancel_receiver: Receiver<bool>, client: Client) -> ProjectLink {
    if project_link.download_links.is_none() {
        return project_link;
    }

    let semaphore = Arc::new(Semaphore::new(200)); // Adjust the number of permits according to your needs
    let mut tasks = Vec::new();

    for link in project_link.download_links.take().unwrap() {
        let client = client.clone();
        let semaphore = Arc::clone(&semaphore);
        let cancel_receiver = cancel_receiver.clone();
        let task = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            get_direct_links_from_download_link(link, recheck_validity, cancel_receiver, client).await
        });
        tasks.push(task);
    }

    let mut updated_links = HashSet::new();
    for task in tasks {
        updated_links.insert(task.await.unwrap());
    }

    project_link.download_links = Some(updated_links);

    project_link
}

const MIRROR_PREFIX: &str = "https://multiup.io/en/mirror/";

async fn get_direct_links_from_download_link(download_link: DownloadLink, recheck_validity: bool, cancel_receiver: Receiver<bool>, client: Client) -> DownloadLink {
    let mirror_link = MIRROR_PREFIX.to_owned() + &download_link.link_id + "/dummy_text";
    if recheck_validity {
        recheck_validity_api(mirror_link, download_link, cancel_receiver, client).await
    } else {
        process_mirror_link(mirror_link.clone(), download_link, cancel_receiver.clone()).await
    }
}

async fn process_mirror_link(mirror_link: String, mut download_link: DownloadLink, cancel_receiver: Receiver<bool>) -> DownloadLink {
    let information = get_mirror_information(&mirror_link, cancel_receiver).await;
    match information {
        Ok((direct_links, link_information)) => {
            download_link.direct_links = Some(direct_links);
            download_link.link_information = Some(link_information);
            download_link.status = Some(Ok(()));
        }
        Err(error) => {
            download_link.status = Some(Err(error));
        }
    }

    download_link
}

static MIRROR_HOSTS_SELECTOR: OnceLock<Selector> = OnceLock::new();
static MIRROR_TITLE_SELECTOR: OnceLock<Selector> = OnceLock::new();
static QUEUE_SELECTOR: OnceLock<Selector> = OnceLock::new();

/// Retrieves
#[async_recursion]
async fn get_mirror_information(mirror_link: &str, cancel_receiver: Receiver<bool>) -> Result<(BTreeSet<DirectLink>, MultiUpLinkInformation), LinkError> {
    let mut direct_links: BTreeSet<DirectLink> = BTreeSet::new();

    let client = Client::new();
    let html = match get_page_html(mirror_link, &client, Some(cancel_receiver), 0).await {
        Ok(html) => html,
        Err(error) => {
            return Err(error);
        }
    };
    let parsed_page = scraper::Html::parse_document(&html);

    let queue_selector = QUEUE_SELECTOR.get_or_init(|| Selector::parse(r#"body > section > div > section > div.row > div > section > div > div > div:nth-child(2) > div > h4"#).unwrap());
    if let Some(_queue_message) = parsed_page.select(queue_selector).next() {
        return Err(LinkError::InQueue);
    }

    let mirror_hosts_selector = MIRROR_HOSTS_SELECTOR.get_or_init(|| Selector::parse(r#"a.host[namehost], button.host[namehost]"#).unwrap());
    for button in parsed_page.select(mirror_hosts_selector) {
        if let Some(direct_link) = get_direct_link_from_button(button) {
            direct_links.insert(direct_link);
        };
    }

    if direct_links.is_empty() {
        return Err(LinkError::NoLinks);
    }

    let file_name_selector = MIRROR_TITLE_SELECTOR.get_or_init(|| Selector::parse(r#"h2.text-truncate"#).unwrap());
    let title = get_title_and_size_from_title_text(parsed_page.select(file_name_selector).next().unwrap());
    let link_information = MultiUpLinkInformation::new_basic(title.0, title.1);

    Ok((direct_links, link_information))
}

fn get_direct_link_from_button(button: ElementRef) -> Option<DirectLink> {
    let button_value = button.value();
    let host_name = button_value.attr("namehost").unwrap();
    let link = button_value.attr("link").unwrap();
    let validity = button_value.attr("validity").unwrap();

    if host_name == "UseNext" {
        None
    } else {
        Some(DirectLink::new(host_name.to_string(), link.to_string(), validity.to_string()))
    }
}

fn get_title_and_size_from_title_text(title: ElementRef) -> (String, String) {
    let mirror_title = title.text().last().unwrap().to_string();
    // Extract the file name
    let file_name = mirror_title.trim_start_matches(" / Mirror list ").split(" (").next().unwrap();
    // Extract the size value and unit
    let size_match = mirror_title
        .trim_end_matches(" )").rsplit(" (")
        .next()
        .unwrap()
        .split_whitespace()
        .collect::<Vec<&str>>();
    let size_value = size_match[0].parse::<f64>().ok().unwrap();
    let size_unit = size_match[1].to_lowercase();


    // Convert size into bytes
    let size_in_bytes = match size_unit.as_str() {
        "b" => size_value as u64,
        "kb" => (size_value * 1024.0) as u64,
        "mb" => (size_value * 1024.0 * 1024.0) as u64,
        "gb" => (size_value * 1024.0 * 1024.0 * 1024.0) as u64,
        _ => 0,
    };

    (file_name.to_string(), size_in_bytes.to_string())
}

fn calculate_row_heights(links: &HashSet<DownloadLink>, filter_menu: &FilterMenu) -> Vec<f32> {
    let mut heights = vec![];

    for link in links {
        let filtered_links = filter_menu.filter_links(link);
        let number_of_direct_links = filtered_links.len() as f32;
        let height = 20.0 + number_of_direct_links * 20.0;

        heights.push(height);
    }

    heights
}