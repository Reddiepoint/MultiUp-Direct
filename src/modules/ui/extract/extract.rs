use crate::modules::config::{read_config, save_config, Config};
use crate::modules::general::{copy_to_clipboard, save_to_file};
use crate::modules::hosting_services::extract::extract_raw_links;
use crate::modules::hosting_services::links::LinkInformation;
use crate::modules::ui::extract::filter::FilterMenu;
use crate::modules::ui::toasts::{new_error_toast, new_info_toast, new_success_toast, new_toasts};
use crossbeam_channel::Receiver;
use eframe::egui;
use eframe::egui::{
    CollapsingHeader, Context, Label, ScrollArea, Sense, TextEdit, TopBottomPanel, Ui, Window,
};
use egui_extras::{Column, TableBuilder};
use egui_toast::Toasts;
use regex::Regex;
use std::collections::{BTreeSet, HashMap};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct ExtractUI {
    toasts: Toasts,
    // Channels
    links_receiver: Option<Receiver<Vec<LinkInformation>>>,
    // Input
    hosting_links: String,
    recheck_validity: bool,
    extracting: bool,
    // Output
    links_information: Vec<LinkInformation>,
    filter_menu: FilterMenu,
    search_query: String,
    selection_range: (Option<(usize, usize)>, Option<(usize, usize)>),
    direct_links: Vec<(String, BTreeSet<String>)>,
    selected_links: HashMap<String, BTreeSet<String>>,
    // Footer
    // Error log
    error_log_open: bool,
    // Passwords
    passwords_open: bool,
    passwords_text: String,
}

impl Default for ExtractUI {
    fn default() -> Self {
        Self {
            toasts: new_toasts(),
            links_receiver: None,
            hosting_links: String::new(),
            recheck_validity: false,
            extracting: false,
            links_information: Vec::new(),
            filter_menu: FilterMenu::default(),
            search_query: String::new(),
            selection_range: (None, None),
            direct_links: Vec::new(),
            selected_links: Default::default(),
            error_log_open: false,
            passwords_open: false,
            passwords_text: match read_config() {
                Ok(config) => config.hosting_passwords.join("\n"),
                Err(_error) => String::new(),
            },
        }
    }
}

impl ExtractUI {
    pub fn display(ctx: &Context, ui: &mut Ui, extract_ui: &mut ExtractUI) {
        // extract_ui.toasts = new_toasts();
        extract_ui.receive_channels();
        extract_ui.display_input_area(ui);
        ui.separator();
        extract_ui.display_footer(ctx);
        extract_ui.display_output_area(ui);
        extract_ui.display_error_log(ctx);
        extract_ui.display_passwords(ctx);
        extract_ui.toasts.show(ctx);
    }

    fn receive_channels(&mut self) {
        if let Some(links_receiver) = &self.links_receiver {
            if let Ok(links_information) = links_receiver.try_recv() {
                self.extracting = false;
                self.links_information = links_information;

                self.filter_menu
                    .update_from_link_information(&self.links_information);

                for link in &self.links_information {
                    if link.status.is_err() {
                        new_error_toast(&mut self.toasts, "Error extracting links.", None, None);
                        return;
                    }
                }
                match self.links_information.is_empty() {
                    true => {
                        new_info_toast(&mut self.toasts, "No links extracted.", None, None);
                        return;
                    }
                    false => new_success_toast(
                        &mut self.toasts,
                        "Successfully extracted links.",
                        None,
                        None,
                    ),
                }
            }
        }
    }

    fn display_input_area(&mut self, ui: &mut Ui) {
        ui.heading("Hosting Service Input Links");

        ScrollArea::both()
            .id_salt("Hosting Service Input Links Box")
            .max_height(ui.available_height() / 4.0)
            .show(ui, |ui| {
                ui.add(
                    TextEdit::multiline(&mut self.hosting_links)
                        .hint_text("Enter your links here")
                        .desired_width(ui.available_width()),
                );
            });

        ui.horizontal(|ui| {
            // Recheck validity checkbox
            // ui.checkbox(&mut self.recheck_validity, "Recheck validity");

            // Extract button
            if ui
                .add_enabled(!self.extracting, egui::Button::new("Extract"))
                .clicked()
            {
                self.extracting = true;
                let (links_sender, links_receiver) = crossbeam_channel::unbounded();
                self.links_receiver = Some(links_receiver);
                // self.extracting = true;
                extract_raw_links(
                    self.hosting_links.clone(),
                    self.recheck_validity,
                    links_sender,
                );
                new_info_toast(&mut self.toasts, "Extracting links...", None, None);
            }

            // Extracting status
            if self.extracting {
                ui.spinner();
                ui.label("Extracting links...");
            }

            if ui.button("See errors").clicked() {
                self.error_log_open = true;
            }

            if ui.button("See passwords").clicked() {
                self.passwords_open = true;
            }
        });
    }

    fn display_output_area(&mut self, ui: &mut Ui) {
        // Process links
        let mut filtered_links_hosts = vec![];
        let mut direct_links = vec![];
        let mut link_table_information = vec![];
        for link_information in self.links_information.iter() {
            if let Ok(()) = link_information.status {
                let filename = match self.search_query.chars().any(|char| char.is_uppercase()) {
                    true => &link_information.filename,
                    false => &link_information.filename.to_lowercase(),
                };

                if let Ok(search_regex) = Regex::new(&self.search_query) {
                    if !search_regex.is_match(filename) {
                        continue;
                    }
                };

                let display_information = format!(
                    "{}\t({} {})",
                    link_information.filename, link_information.size, link_information.size_unit
                );

                // TODO: Add interaction with links
                let links = link_information
                    .direct_links
                    .iter()
                    .filter(|link| self.filter_menu.filter_link(link, true, true))
                    .map(|link| link.link.clone())
                    .collect::<BTreeSet<String>>();

                direct_links.push((link_information.id.clone(), links.clone()));
                let links: Vec<String> = links.into_iter().collect();

                let mut hosts = link_information
                    .direct_links
                    .iter()
                    .filter(|link| self.filter_menu.filter_link(link, false, true))
                    .map(|link| link.host.clone())
                    .collect::<Vec<String>>();

                filtered_links_hosts.append(&mut hosts);
                link_table_information.push((display_information, link_information, links));
            }
        }
        self.direct_links = direct_links;
        self.filter_menu
            .update_from_link_list(&filtered_links_hosts);

        ui.heading("Extracted Links");

        // Filename search
        ui.horizontal(|ui| {
            ui.add(
                TextEdit::singleline(&mut self.search_query)
                    .id_salt("Filename Search Filter")
                    .hint_text("Search... (supports regex and smart case)"),
            );

            if ui.button("Clear").clicked() {
                self.search_query = String::new();
            }
        });

        // Link buttons
        let mut select_all = false;
        let mut deselect_all = false;
        enum ToastType {
            CopyAll,
            CopySelected,
            FailedtoSave(String),
            SaveAllAsTXT(String),
            SaveSelectedAsTXT(String),
            NoLinks,
            Nothing,
        }
        let mut toast_type = ToastType::Nothing;
        ui.horizontal(|ui| {
            if ui.button("Copy all").clicked() {
                if self.direct_links.is_empty() {
                    toast_type = ToastType::NoLinks;
                    return;
                }
                self.copy_all_links(ui);
                toast_type = ToastType::CopyAll;
            }

            if ui.button("Copy selected").clicked() {
                if self.selected_links.is_empty() {
                    toast_type = ToastType::NoLinks;
                    return;
                }
                self.copy_selected_links(ui);
                toast_type = ToastType::CopySelected;
            }

            if ui.button("Select all").clicked() {
                select_all = true;
            }

            if ui.button("Deselect all").clicked() {
                deselect_all = true;
            }

            if ui.button("Save all as TXT").clicked() {
                if self.direct_links.is_empty() {
                    toast_type = ToastType::NoLinks;
                    return;
                }
                let mut links_text = String::new();
                for (id, links) in &self.direct_links {
                    let links = links.iter().cloned().collect::<Vec<String>>();
                    if links.is_empty() {
                        continue;
                    }
                    let filename = &link_table_information
                        .iter()
                        .find(|(_display, link_information, _links)| &link_information.id == id)
                        .unwrap()
                        .1
                        .filename;
                    links_text = format!("{}// {}\n{}\n", links_text, filename, links.join("\n"));
                }
                let current_time = SystemTime::now();
                let time_stamp = current_time
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_secs();
                let text_file_name = format!("extracted_links_{}.txt", time_stamp);
                if links_text.is_empty() {
                    toast_type = ToastType::NoLinks;
                    return;
                }
                if let Err(error) = save_to_file(&text_file_name, &links_text) {
                    toast_type = ToastType::FailedtoSave(error);
                    return;
                };
                toast_type = ToastType::SaveAllAsTXT(text_file_name);
            }

            if ui.button("Save selected as TXT").clicked() {
                if self.selected_links.is_empty() {
                    toast_type = ToastType::NoLinks;
                    return;
                }
                let mut links_text = String::new();
                for (id, links) in &self.selected_links {
                    let links = links.iter().cloned().collect::<Vec<String>>();
                    let filename = &link_table_information
                        .iter()
                        .find(|(_display, link_information, _links)| &link_information.id == id)
                        .unwrap()
                        .1
                        .filename;
                    links_text = format!("{}// {}\n{}\n", links_text, filename, links.join("\n"));
                }
                let current_time = SystemTime::now();
                let time_stamp = current_time
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_secs();
                let text_file_name = format!("extracted_links_{}.txt", time_stamp);
                if let Err(error) = save_to_file(&text_file_name, &links_text) {
                    toast_type = ToastType::FailedtoSave(error);
                    return;
                };
                toast_type = ToastType::SaveSelectedAsTXT(text_file_name);
            }
        });

        if select_all {
            self.selected_links = self.direct_links.iter().cloned().collect();
        } else if deselect_all {
            self.selected_links.clear();
        }
        match toast_type {
            ToastType::CopyAll => new_info_toast(
                &mut self.toasts,
                "Copied all links to clipboard.",
                None,
                None,
            ),
            ToastType::CopySelected => new_info_toast(
                &mut self.toasts,
                "Copied selected links to clipboard.",
                None,
                None,
            ),
            ToastType::FailedtoSave(error) => new_error_toast(
                &mut self.toasts,
                format!("Failed to save links to file: {}", error).as_str(),
                None,
                None,
            ),
            ToastType::SaveAllAsTXT(filename) => new_info_toast(
                &mut self.toasts,
                format!("Saved all links to {}.", filename).as_str(),
                None,
                None,
            ),
            ToastType::SaveSelectedAsTXT(filename) => new_info_toast(
                &mut self.toasts,
                format!("Saved selected links to {}.", filename).as_str(),
                None,
                None,
            ),
            ToastType::NoLinks => new_info_toast(&mut self.toasts, "No links found.", None, None),
            ToastType::Nothing => {}
        }
        let mut link_labels = vec![];
        let height = ui.available_height() - 20.0;
        // Links and filter
        // Links takes up 80% of width
        ui.horizontal(|ui| {
            ui.set_height(height);
            ScrollArea::vertical()
                .id_salt("Link Information Box")
                .max_width(0.8 * ui.available_width())
                .max_height(ui.available_height())
                .show(ui, |ui| {
                    let heights = link_table_information
                        .iter()
                        .map(|(_name, _original_link, links)| 20.0 + links.len() as f32 * 18.0)
                        .collect::<Vec<f32>>();

                    ui.push_id("Link Information Table", |ui| {
                        let width = ui.available_width();
                        TableBuilder::new(ui)
                            .column(Column::exact(width))
                            .body(|body| {
                                body.heterogeneous_rows(heights.into_iter(), |mut row| {
                                    let row_index = row.index();
                                    row.col(|ui| {
                                        CollapsingHeader::new(&link_table_information[row_index].0)
                                            .id_salt(&link_table_information[row_index].1.id)
                                            .default_open(true)
                                            .open(Some(true))
                                            .show(ui, |ui| {
                                                let links = &link_table_information[row_index].2;
                                                let id = &link_table_information[row_index].1.id;
                                                for link in links {
                                                    let mut label = ui.add(
                                                        Label::new(link)
                                                            .selectable(false)
                                                            .sense(Sense::click()),
                                                    );
                                                    if let Some(links) = self.selected_links.get(id)
                                                    {
                                                        if links.contains(link) {
                                                            label = label.highlight();
                                                        }
                                                    }

                                                    link_labels.push((label, id, link));
                                                }
                                            });
                                    });
                                });
                            });
                    });
                });

            self.filter_menu.show(ui);
        });

        // Link interactions
        for (label, id, link) in link_labels {
            // Selection
            if label.clicked() {
                let (control_is_down, shift_is_down) =
                    ui.ctx().input(|ui| (ui.modifiers.ctrl, ui.modifiers.shift));
                if control_is_down {
                    let mut selected_direct_links =
                        self.selected_links.get(id).cloned().unwrap_or_default();
                    match selected_direct_links.contains(link) {
                        true => {
                            selected_direct_links.remove(link);
                            self.selected_links
                                .insert(id.to_string(), selected_direct_links.clone());
                        }
                        false => {
                            selected_direct_links.insert(link.to_string());
                            self.selected_links
                                .insert(id.to_string(), selected_direct_links.clone());
                        }
                    };
                    self.selection_range = (None, None);
                } else if shift_is_down {
                    let mut index_of_click = (0, 0);
                    for (id_index, direct_links) in self.direct_links.iter().enumerate() {
                        for (link_index, direct_link) in direct_links.1.iter().enumerate() {
                            if direct_link == link {
                                index_of_click = (id_index, link_index)
                            }
                        }
                    }
                    match self.selection_range.0 {
                        None => {
                            self.selection_range.0 = Some(index_of_click);
                        }
                        Some(start) => {
                            if start.0 > index_of_click.0
                                || start.0 == index_of_click.0 && start.1 > index_of_click.1
                            {
                                self.selection_range.0 = Some(index_of_click);
                                self.selection_range.1 = Some(start);
                            } else {
                                self.selection_range.1 = Some(index_of_click);
                            }
                        }
                    }
                } else {
                    let mut index = (0, 0);
                    for (id_index, direct_links) in self.direct_links.iter().enumerate() {
                        for (link_index, direct_link) in direct_links.1.iter().enumerate() {
                            if direct_link == link {
                                index = (id_index, link_index)
                            }
                        }
                    }
                    self.selection_range = (Some(index), None);
                }

                if self.selection_range.0.is_some() && self.selection_range.1.is_some() {
                    let start = self.selection_range.0.unwrap();
                    let end = self.selection_range.1.unwrap();
                    for (mut i, (id, links)) in
                        self.direct_links[start.0..=end.0].iter().enumerate()
                    {
                        i += start.0;
                        let mut selected_links =
                            self.selected_links.get(id).cloned().unwrap_or_default();
                        let links: Vec<String> = links.iter().cloned().collect();
                        let mut links_to_insert = Vec::new();
                        if i == start.0 && i == end.0 {
                            links_to_insert = links[start.1..=end.1].to_vec();
                        } else if i == start.0 {
                            links_to_insert = links[start.1..].to_vec();
                        } else if i == end.0 {
                            links_to_insert = links[..=end.1].to_vec();
                        } else if start.0 < i && i < end.0 {
                            links_to_insert = links.to_vec();
                        }
                        for link in links_to_insert {
                            selected_links.insert(link);
                        }
                        self.selected_links
                            .insert(id.to_string(), selected_links.clone());
                    }
                }
            }

            // Context menu
            label.context_menu(|ui| {
                if ui.button("Copy link").clicked() {
                    copy_to_clipboard(ui, link.to_string());
                }

                if !self.selected_links.is_empty() && ui.button("Copy selected links").clicked() {
                    self.copy_selected_links(ui);
                }

                if ui.button("Copy all links").clicked() {
                    self.copy_all_links(ui);
                }
            });
        }
    }

    fn display_footer(&mut self, ctx: &Context) {
        TopBottomPanel::bottom("Information Footer").show(ctx, |ui| {
            let successful_links = self
                .links_information
                .iter()
                .filter(|link| link.status.is_ok())
                .count();
            let selected_links = self
                .selected_links
                .values()
                .map(|links| links.len())
                .sum::<usize>();
            let direct_links = self
                .direct_links
                .iter()
                .map(|(_id, links)| links.len())
                .sum::<usize>();
            ui.label(format!(
                "Extracted links: {}/{} | Selected links: {}/{}",
                successful_links,
                self.links_information.len(),
                selected_links,
                direct_links
            ));
        });
    }

    pub fn display_error_log(&mut self, ctx: &Context) {
        Window::new("Extract Error Log")
            .open(&mut self.error_log_open)
            .default_width(1000.0)
            .show(ctx, |ui| {
                let error_log: Vec<(String, String)> = self
                    .links_information
                    .iter()
                    .filter(|link| link.status.is_err())
                    .map(|link| {
                        (
                            link.original_link.clone(),
                            link.status.clone().unwrap_err().get_error_message(),
                        )
                    })
                    .collect();
                ScrollArea::both()
                    .id_salt("Extract Error Log Box")
                    .max_width(ui.available_width() * 0.8)
                    .show(ui, |ui| {
                        let heights = error_log
                            .iter()
                            .map(|(_link, _error)| 20.0 + 18.0)
                            .collect::<Vec<f32>>();
                        ui.push_id("Extract Error Log Table", |ui| {
                            let width = ui.available_width() / 2.0;
                            TableBuilder::new(ui)
                                .column(Column::exact(width))
                                .column(Column::remainder())
                                .body(|body| {
                                    body.heterogeneous_rows(heights.into_iter(), |mut row| {
                                        let row_index = row.index();
                                        row.col(|ui| {
                                            ui.label(&error_log[row_index].0);
                                        });
                                        row.col(|ui| {
                                            ui.label(&error_log[row_index].1);
                                        });
                                    })
                                });
                        });
                    });
            });
    }

    pub fn display_passwords(&mut self, ctx: &Context) {
        Window::new("Mirror Passwords")
            .open(&mut self.passwords_open)
            .show(ctx, |ui| {
                if ui.button("Save passwords").clicked() {
                    let mut config = match read_config() {
                        Ok(config) => config,
                        Err(error) => {
                            new_error_toast(
                                &mut self.toasts,
                                format!("Error reading config: {}", error.get_error_message())
                                    .as_str(),
                                None,
                                None,
                            );
                            Config::default()
                        }
                    };

                    let passwords = self
                        .passwords_text
                        .split("\n")
                        .filter(|password| !password.is_empty())
                        .map(|password| password.to_owned())
                        .collect::<Vec<String>>();

                    config.hosting_passwords = passwords;

                    match save_config(&config) {
                        Ok(_) => {
                            new_success_toast(
                                &mut self.toasts,
                                "Successfully saved passwords.",
                                None,
                                None,
                            );
                        }
                        Err(error) => {
                            new_error_toast(
                                &mut self.toasts,
                                format!("Error saving config: {}", error.get_error_message())
                                    .as_str(),
                                None,
                                None,
                            );
                        }
                    }
                }
                ui.add(
                    TextEdit::multiline(&mut self.passwords_text)
                        .hint_text("Enter your passwords here, on each line")
                        .desired_width(ui.available_width()),
                );
            });
    }
    fn copy_selected_links(&self, ui: &Ui) {
        let links: Vec<String> = self
            .selected_links
            .iter()
            .flat_map(|(id, links)| links)
            .cloned()
            .collect();
        copy_to_clipboard(ui, links.join("\n"));
    }

    fn copy_all_links(&self, ui: &Ui) {
        let links: Vec<String> = self
            .direct_links
            .iter()
            .flat_map(|(id, link)| link)
            .cloned()
            .collect();
        copy_to_clipboard(ui, links.join("\n"));
    }
}
