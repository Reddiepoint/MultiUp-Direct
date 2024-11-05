use std::collections::BTreeSet;
use crossbeam_channel::Receiver;
use eframe::egui;
use eframe::egui::Direction::TopDown;
use eframe::egui::{CollapsingHeader, Context, Label, ScrollArea, Sense, TextEdit, Ui};
use egui::Align2;
use egui_extras::{Column, TableBuilder};
use egui_toast::Toasts;

use crate::modules::hosting_services::extract::extract_raw_links;
use crate::modules::hosting_services::filter::filter_link;
use crate::modules::hosting_services::links::LinkInformation;
use crate::modules::ui::extract::filter::FilterMenu;

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

                self.filter_menu.update(&self.links_information);
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
            ui.checkbox(&mut self.recheck_validity, "Recheck validity");

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
        ui.heading("Extracted Links");

        // Filename search
        ui.horizontal(|ui| {
            ui.add(
                TextEdit::singleline(&mut self.search_query)
                    .id_salt("Filename Search Filter")
                    .hint_text("Type to search for specific files"),
            );

            if ui.button("Clear").clicked() {
                self.search_query = String::new();
            }
        });
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
                    let mut link_table_information = vec![];
                    for link_information in &self.links_information {
                        if let Ok(()) = link_information.status {
                            // TODO: Add search logic (possibly regex)
                            let display_information = format!(
                                "{}\t({} {})",
                                link_information.filename,
                                link_information.size,
                                link_information.size_unit
                            );

                            // TODO: Add interaction with links
                            link_table_information.push((
                                display_information,
                                link_information.original_link.clone(),
                                link_information
                                    .direct_links
                                    .iter()
                                    .filter(|link| self.filter_menu.filter_link(link))
                                    .map(|link| link.link.clone())
                                    .collect::<BTreeSet<String>>(),
                            ));
                        }
                    }
                    
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
                                            .id_salt(&link_table_information[row_index].1)
                                            .default_open(true)
                                            .open(Some(true))
                                            .show(ui, |ui| {
                                                for link in &link_table_information[row_index].2 {
                                                    ui.add(Label::new(link).sense(Sense::click()));
                                                }
                                            });
                                    });
                                });
                            });
                    });
                });

            self.filter_menu.show(ui);
        });
        // TODO: Write output links
    }

    fn display_footer(&mut self, ctx: &Context) {}
    pub fn display_error_log(&mut self, ctx: &Context) {}
}
