use std::collections::HashSet;
use std::sync::mpsc;
use eframe::egui::{Label, ScrollArea, Sense, TextEdit, Ui};
use tokio::runtime::Runtime;


use crate::functions::download::generate_direct_links;
use crate::functions::filter::filter_links;
use crate::structs::filter::FilterMenu;
use crate::structs::hosts::Link;


#[derive(Clone)]
pub struct Download {
    pub mirror_links_input: String,
    pub check_status: bool,
    pub direct_links: Vec<Link>,
    pub links_to_display: Vec<String>,
    pub filter: FilterMenu,
    pub selected_links: Vec<String>,
    pub index_1: usize,
    pub index_2: usize,
}

impl Default for Download {
    fn default() -> Self {
        Download {
            mirror_links_input: String::new(),
            check_status: true,
            direct_links: vec![],
            links_to_display: vec![],
            filter: FilterMenu::default(),
            selected_links: vec![],
            index_1: 0,
            index_2: 0,
        }
    }
}

impl Download {
    fn mirror_links_box(&mut self, ui: &mut Ui) {
        let height = ui.available_height() / 2.0;
        ui.label("Enter your MultiUp links:");
        ui.vertical(|ui| {
            ui.set_max_height(height);
            ScrollArea::vertical().min_scrolled_height(ui.available_height()).show(ui, |ui| {
                ui.add(TextEdit::multiline(&mut self.mirror_links_input)
                    .hint_text("Enter your MultiUp links separated by a new line\nSupports short and long links")
                    .desired_width(ui.available_width())
                );
            });
        });
    }

    fn generate_links_ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Check status
            ui.checkbox(
                &mut self.check_status,
                "Re-check host status",
            );

            // Generate links
            if ui.button("Generate links").clicked() {
                let rt = Runtime::new().expect("Unable to create runtime");
                let _ = rt.enter();
                let (tx, rx) = mpsc::channel();
                // Create runtime
                let mirror_links = self.mirror_links_input.clone();
                let check_status = self.check_status;

                std::thread::spawn(move || {
                    let result = rt.block_on(async {
                        generate_direct_links(&mirror_links, check_status).await
                    });

                    match tx.send(result) {
                        Ok(_) => {}
                        Err(error) => {println!("Error: {}", error)}
                    };

                });

                let (generated_links, links_host) = rx.recv().unwrap();
                self.direct_links = generated_links;
                self.filter.hosts = links_host;
            };


        });
    }

    fn display_direct_links(&mut self, ui: &mut Ui) {
        let height = ui.available_height();
        self.links_to_display = filter_links(&self.direct_links, &self.filter);
        // Generated direct links output
        ui.horizontal(|ui| {
            ui.set_height(height);
            ui.vertical(|ui| {
                ui.label("Direct links: ");
                ScrollArea::vertical()
                    .min_scrolled_height(ui.available_height())
                    .id_source("Display links")
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width() - 200.0);
                        ui.vertical(|ui| {
                            let mut selected_links: HashSet<&str> = self.selected_links.iter().map(|url| url.as_str()).collect();
                            for link in self.selected_links.iter() {
                                if !self.links_to_display.contains(link) {
                                    selected_links.remove(link.as_str());
                                };
                            };
                            for link in self.links_to_display.iter() {
                                let output = ui.add(Label::new(link).sense(Sense::click()));
                                if output.hovered() || self.selected_links.contains(link) {
                                    output.clone().highlight();
                                };

                                let control_down = ui.ctx().input(|ui| {
                                    ui.modifiers.ctrl
                                });

                                let shift_down = ui.ctx().input(|ui| {
                                    ui.modifiers.shift
                                });

                                if output.clicked() && control_down {
                                    if selected_links.contains(link.as_str()) {
                                        selected_links.remove(link.as_str());
                                    } else {
                                        selected_links.insert(link);
                                    }
                                } else if output.clicked() && shift_down {
                                    if self.index_1 == 0 {
                                        self.index_1 = self.links_to_display.iter().position(|url| url == link).unwrap() + 1;
                                        self.index_2 = self.links_to_display.iter().position(|url| url == link).unwrap() + 1;
                                    } else {
                                        self.index_2 = self.links_to_display.iter().position(|url| url == link).unwrap() + 1;
                                    };
                                } else if output.clicked() {
                                    self.index_1 = self.links_to_display.iter().position(|url| url == link).unwrap() + 1;
                                    self.index_2 = self.links_to_display.iter().position(|url| url == link).unwrap() + 1;
                                };

                                let slice = &self.links_to_display;
                                let index_1 = self.index_1;
                                let index_2 = self.index_2;
                                if self.index_1 > self.index_2 {
                                    self.index_1 = index_2;
                                    self.index_2 = index_1;
                                };

                                if self.index_1 != 0 && self.index_1 != self.index_2 {
                                    slice[self.index_1-1..self.index_2].iter().for_each(|link| { selected_links.insert(link); });
                                    if ui.ctx().input(|ui| {
                                        !ui.modifiers.shift
                                    }) {
                                        self.index_1 = 0;
                                    };


                                    //self.index_2 = 0;
                                };

                                output.context_menu(|ui| {
                                    if ui.button("Copy link").clicked() {
                                        ui.output_mut(|output| output.copied_text = link.to_string());
                                        ui.close_menu();
                                    };
                                    if !self.selected_links.is_empty() && ui.button("Copy selected links").clicked() {
                                        ui.output_mut(|output| output.copied_text = self.selected_links.join("\n"));
                                        ui.close_menu();
                                    };
                                    if ui.button("Copy all links").clicked() {
                                        ui.output_mut(|output| output.copied_text = self.links_to_display.join("\n"));
                                        ui.close_menu();
                                    };
                                    ui.separator();
                                    if ui.button("Open link in browser").clicked() {
                                        let _ = webbrowser::open(link);
                                        ui.close_menu();
                                    };
                                    if !self.selected_links.is_empty() && ui.button("Open selected links in browser").clicked() {
                                        for link in self.selected_links.iter() {
                                            let _ = webbrowser::open(link);
                                        }
                                        ui.close_menu();
                                    };
                                    if ui.button("Open all links in browser").clicked() {
                                        for link in self.links_to_display.iter() {
                                            let _ = webbrowser::open(link);
                                        }
                                        ui.close_menu();
                                    };

                                    if !self.selected_links.is_empty() {
                                        ui.separator();
                                        if ui.button("Deselect all links").clicked() {
                                            selected_links = HashSet::new();
                                            ui.close_menu();
                                        }
                                    }
                                });
                            };
                            self.selected_links = selected_links.iter().map(|url| url.to_string()).collect();
                        });
                    });
            });

            FilterMenu::show(ui, &mut self.filter);
        });
    }


    pub fn show(ui: &mut Ui, download: &mut Download) {
        // Enter download links box
        download.mirror_links_box(ui);
        // Check hosts and generate button
        download.generate_links_ui(ui);
        // Output and filter menu
        download.display_direct_links(ui);
    }
}
