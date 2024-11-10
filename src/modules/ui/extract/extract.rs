use crate::modules::hosting_services::extract::extract_raw_links;
use crate::modules::hosting_services::links::LinkInformation;
use crate::modules::other::{copy_to_clipboard, save_to_file};
use crate::modules::ui::extract::filter::FilterMenu;
use crossbeam_channel::Receiver;
use eframe::egui;
use eframe::egui::Direction::TopDown;
use eframe::egui::{
    CollapsingHeader, Context, Label, ScrollArea, Sense, TextEdit, TopBottomPanel, Ui,
};
use egui::Align2;
use egui_extras::{Column, TableBuilder};
use egui_toast::Toasts;
use regex::Regex;
use std::collections::{BTreeSet, HashMap};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Default)]
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
}

impl ExtractUI {
    pub fn display(ctx: &Context, ui: &mut Ui, extract_ui: &mut ExtractUI) {
        extract_ui.toasts = Toasts::new()
            .anchor(Align2::RIGHT_TOP, (10.0, 10.0))
            .direction(TopDown);
        extract_ui.toasts.show(ctx);
        extract_ui.receive_channels();
        extract_ui.display_input_area(ui);
        ui.separator();
        extract_ui.display_footer(ctx);
        extract_ui.display_output_area(ui);
    }

    fn receive_channels(&mut self) {
        if let Some(links_receiver) = &self.links_receiver {
            if let Ok(links_information) = links_receiver.try_recv() {
                self.extracting = false;
                self.links_information = links_information;
                for link in &self.links_information {
                    println!("{}", link.filename);
                    println!(
                        "{}",
                        link.direct_links
                            .iter()
                            .map(|link| link.link.clone())
                            .collect::<Vec<String>>()
                            .join("\n")
                    );
                }

                self.filter_menu
                    .update_from_link_information(&self.links_information);
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
                let (links_sender, links_receiver) = crossbeam_channel::unbounded();
                self.links_receiver = Some(links_receiver);
                // self.extracting = true;
                extract_raw_links(
                    self.hosting_links.clone(),
                    self.recheck_validity,
                    links_sender,
                );
            }

            // Extracting status
            if self.extracting {
                ui.spinner();
                ui.label("Extracting links...");
            }
        });
    }

    fn display_output_area(&mut self, ui: &mut Ui) {
        // Process links
        let mut filtered_links_hosts = vec![];
        let mut direct_links = vec![];
        let mut link_table_information = vec![];
        for link_information in &self.links_information {
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
                link_table_information.push((
                    display_information,
                    link_information,
                    links,
                ));
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
        ui.horizontal(|ui| {
            if ui.button("Copy all").clicked() {
                self.copy_all_links(&ui);
            }

            if ui.button("Copy selected").clicked() {
                self.copy_selected_links(&ui);
            }

            if ui.button("Select all").clicked() {
                select_all = true;
            }

            if ui.button("Deselect all").clicked() {
                deselect_all = true;
            }

            if ui.button("Save all as TXT").clicked() {
                // TODO: Group links by ID
                // let text = self.direct_links.iter().map(|a|).join("\n");
                let mut text = String::new();
                for (id, links) in &self.direct_links {
                    let links = links.iter().cloned().collect::<Vec<String>>();
                    let filename = &link_table_information.iter()
                        .find(|(_display, link_information, _links)| &link_information.id == id)
                        .unwrap()
                        .1
                        .filename;
                    text = format!("{}// {}\n{}\n", text, filename, links.join("\n"));
                }
                let current_time = SystemTime::now();
                let time_stamp = current_time
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_secs();
                println!("{:?}", time_stamp);
                save_to_file(
                    format!("extracted_links_{}.txt", time_stamp).as_str(),
                    &text,
                );
            }

            if ui.button("Save selected as TXT").clicked() {
                let mut text = String::new();
                for (id, links) in &self.selected_links {
                    let links = links.iter().cloned().collect::<Vec<String>>();
                    let filename = &link_table_information.iter()
                        .find(|(_display, link_information, _links)| &link_information.id == id)
                        .unwrap()
                        .1
                        .filename;
                    text = format!("{}// {}\n{}\n", text, filename, links.join("\n"));
                }
                let current_time = SystemTime::now();
                let time_stamp = current_time
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_secs();
                println!("{:?}", time_stamp);
                save_to_file(
                    format!("extracted_links_{}.txt", time_stamp).as_str(),
                    &text,
                );
            }
        });

        if select_all {
            self.selected_links = self.direct_links.iter().cloned().collect();
        } else if deselect_all {
            self.selected_links.clear();
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
                                                    if let Some(links) = self.selected_links.get(id) {
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
                    let mut selected_direct_links = self.selected_links.get(id).cloned().unwrap_or_default();
                    match selected_direct_links.contains(link) {
                        true => {
                            selected_direct_links.remove(link);
                            self.selected_links.insert(id.to_string(), selected_direct_links.clone());
                        }
                        false => {
                            selected_direct_links.insert(link.to_string());
                            self.selected_links.insert(id.to_string(), selected_direct_links.clone());
                        }
                    };
                    self.selection_range = (None, None);
                } else if shift_is_down {
                    let mut index = (0, 0);
                    for (id_index, direct_links) in self.direct_links.iter().enumerate() {
                        for (link_index, direct_link) in direct_links.1.iter().enumerate() {
                            if direct_link == link {
                                index = (id_index, link_index)
                            }
                        }
                    }
                    // let index = self
                    //     .direct_links
                    //     .iter()
                    //     .flat_map(|(id, links)| links)
                    //     .position(|direct_link| direct_link == link)
                    //     .unwrap();

                    match self.selection_range.0 {
                        None => {
                            self.selection_range.0 = Some(index);
                        }
                        Some(start) => {
                            if start.0 >= index.0 || start.0 == index.0 && start.1 > index.1 {
                                self.selection_range.0 = Some(index);
                                self.selection_range.1 = Some(start);
                            } else {
                                self.selection_range.1 = Some(index);
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
                    for (mut i, (id, links)) in self.direct_links[start.0..=end.0].iter().enumerate() {
                        i += start.0;
                        let mut selected_links = self.selected_links.get(id).cloned().unwrap_or_default();
                        let links: Vec<String> = links.iter().cloned().collect();
                        let mut links_to_insert = Vec::new();
                        
                        if i == start.0 && i == end.0 {
                            links_to_insert = links[start.1..end.1].to_vec();
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
                        self.selected_links.insert(id.to_string(), selected_links.clone());
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
            ui.label(format!(
                "Extracted links: {} | Selected links: {}/{}",
                self.links_information.len(),
                self.selected_links.len(),
                self.direct_links.len()
            ));
        });
    }

    pub fn display_error_log(&mut self, ctx: &Context) {}

    fn copy_selected_links(&self, ui: &Ui) {
        let links: Vec<String> = self.selected_links.iter().flat_map(|(id, links)| links).cloned().collect();
        copy_to_clipboard(ui, links.join("\n"));
    }

    fn copy_all_links(&self, ui: &Ui) {
        let links: Vec<String> = self.direct_links.iter().flat_map(|(id, link)| link).cloned().collect();
        copy_to_clipboard(ui, links.join("\n"));
    }
}
