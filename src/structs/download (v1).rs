use std::collections::HashSet;

use std::time::Instant;
use crossbeam_channel::{Receiver, SendError};
use eframe::egui::{Button, Label, ScrollArea, Sense, TextEdit, Ui};

use tokio::runtime::Runtime;


use crate::functions::download::{fix_mirror_links, generate_direct_links};
use crate::functions::filter::filter_links;
use crate::structs::filter::FilterMenu;
use crate::structs::hosts::{Link, LinkValidityResponse};


#[derive(Clone)]
pub struct Download {
    pub multiup_links: String,
    pub check_status: bool,
    pub mirror_links: Vec<Link>,
    pub links_to_display: Vec<String>,
    pub filter: FilterMenu,
    pub selected_links: Vec<String>,
    pub link_information: Vec<LinkValidityResponse>,
    pub link_info_receiver: Option<Receiver<Vec<LinkValidityResponse>>>,
    pub index_1: usize,
    pub index_2: usize,
    pub receiver: Option<Receiver<(Vec<Link>, Vec<(String, bool)>)>>,
    pub timer_start: Option<Instant>,
    pub generation_time_receiver: Option<Receiver<u128>>,
    pub generation_time: u128,
    pub generation_status_receiver: Option<Receiver<bool>>,
    pub generation_status: Option<bool>,
    pub number_of_links_receiver: Option<Receiver<usize>>,
    pub total_links: usize,
    pub number_of_generated_links_receiver: Option<Receiver<u8>>,
    pub number_of_generated_links: usize,
    //pub emergency_stop: bool
}

impl Default for Download {
    fn default() -> Self {
        Download {
            multiup_links: String::new(),
            check_status: true,
            mirror_links: vec![],
            links_to_display: vec![],
            filter: FilterMenu::default(),
            selected_links: vec![],
            link_information: vec![],
            link_info_receiver: None,
            index_1: 0,
            index_2: 0,
            receiver: None,
            timer_start: None,
            generation_time_receiver: None,
            generation_time: 0,
            generation_status_receiver: None,
            generation_status: None,
            //emergency_stop: false,
            number_of_links_receiver: None,
            total_links: 0,
            number_of_generated_links_receiver: None,
            number_of_generated_links: 0,
        }
    }
}

impl Download {
    fn mirror_links_box(&mut self, ui: &mut Ui) {
        let height = ui.available_height() / 2.0;
        ui.label("Enter your MultiUp links:");
        ui.vertical(|ui| {
            ui.set_max_height(height);
            ScrollArea::vertical().id_source("Input Box").max_height(ui.available_height() / 2.0).min_scrolled_height(ui.available_height() / 2.0).show(ui, |ui| {
                ui.add(TextEdit::multiline(&mut self.multiup_links)
                    .hint_text("Enter your MultiUp links separated by a new line\nSupports short and long links")
                    .desired_width(ui.available_width())
                );
            });
            if !self.link_information.is_empty() {
                ui.collapsing("Link Information", |ui| {
                    ScrollArea::vertical().id_source("File Info").min_scrolled_height(ui.available_height()).show(ui, |ui| {
                        for file in self.link_information.iter() {
                            let info = if let Some(description) = &file.description {
                                format!("{} | {} ({} bytes). Uploaded {} ({} seconds). Total downloads: {}",
                                        file.file_name,
                                        description,
                                        file.size,
                                        file.date_upload,
                                        file.time_upload,
                                        file.number_downloads,
                                )

                            } else {
                                format!("{} ({} bytes). Uploaded {} ({} seconds). Total downloads: {}",
                                        file.file_name,
                                        file.size,
                                        file.date_upload,
                                        file.time_upload,
                                        file.number_downloads,
                                )
                            };
                            ui.label(info);

                        }
                    });
                });
            }
        });
    }

    fn generate_links_ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Check status
            ui.checkbox(&mut self.check_status, "Re-check host status");

            let button = if let Some(status) = self.generation_status {
                ui.add_enabled(!status, Button::new("Generate links"))
            } else {
                ui.add(Button::new("Generate links"))
            };

            // Generate links
            if button.clicked() {
                // Create runtime
                let mirror_links = self.multiup_links.clone();
                let check_status = self.check_status;
                let rt = Runtime::new().expect("Unable to create runtime");
                let _ = rt.enter();

                // Create channels for communication
                let (link_tx, link_rx) = crossbeam_channel::unbounded();
                let (status_tx, status_rx) = crossbeam_channel::unbounded();
                let (time_tx, time_rx) = crossbeam_channel::unbounded();
                let (number_of_links_tx, number_of_links_rx) = crossbeam_channel::unbounded();
                let (number_of_generated_links_tx, number_of_generated_links_rx) = crossbeam_channel::unbounded();
                let (link_info_tx, link_info_rx) = crossbeam_channel::unbounded();

                // Store the receivers in fields to use later
                self.receiver = Some(link_rx);
                self.link_information = vec![];
                self.link_info_receiver = Some(link_info_rx);
                self.generation_time_receiver = Some(time_rx);
                self.generation_status_receiver = Some(status_rx);
                self.number_of_links_receiver = Some(number_of_links_rx);
                self.total_links = 0;
                self.number_of_generated_links_receiver = Some(number_of_generated_links_rx);
                self.number_of_generated_links = 0;

                // Send initial values
                time_tx.send(0);
                status_tx.send(true);

                self.timer_start = Some(Instant::now());
                // Spawn a thread to generate direct links
                std::thread::spawn(move || {
                    let result = rt.block_on(async {
                        let links = fix_mirror_links(&mirror_links); // All links are ok
                        number_of_links_tx.send(links.len());
                        generate_direct_links(links, check_status, number_of_generated_links_tx, link_info_tx).await
                    });
                    link_tx.send(result).expect("Unable to send result");
                    status_tx.send(false);
                });
            }

            // Show generation status and time
            if let Some(generating) = self.generation_status {
                if generating {
                    ui.spinner();
                    ui.label("Generating...");
                } else {
                    ui.label("Generated!");
                };
            };

            // Update fields from receivers
            if let Some(rx) = &self.link_info_receiver {
                if let Ok(info) = rx.try_recv() {
                    self.link_information = info;
                }
            };
            if let Some(rx) = &self.number_of_links_receiver {
                if let Ok(size) = rx.try_recv() {
                    self.total_links = size;
                }
            };

            if let Some(rx) = &self.number_of_generated_links_receiver {
                while let Ok(generated) = rx.try_recv() {
                    self.number_of_generated_links += generated as usize;
                }
            };

            if let Some(rx) = &self.generation_status_receiver {
                if let Ok(generating) = rx.try_recv() {
                    self.generation_status = Some(generating);
                    if !generating {
                        self.timer_start.take();
                    }
                }
            }

            if let Some(time) = self.timer_start {
                if self.generation_status == Some(true) {
                    self.generation_time = time.elapsed().as_millis();
                }
            };

            if let Some(rx) = &self.generation_time_receiver {
                if let Ok(time) = rx.try_recv() {
                    self.generation_time = time;
                }
            }

            if self.generation_time != 0 {
                let formatted_time = format!("Time taken: {}.{}s", self.generation_time / 1000, self.generation_time % 1000);
                ui.label(formatted_time);
                let formatted_progress = format!("{}/{} completed.", self.number_of_generated_links, self.total_links);
                ui.label(formatted_progress);
            }



            if let Some(rx) = &self.receiver {
                if let Ok((generated_links, links_hosts)) = rx.try_recv() {
                    self.mirror_links = generated_links;
                    self.filter.hosts = links_hosts;
                    self.receiver = None;
                }
            }
        });
    }

    fn display_direct_links(&mut self, ui: &mut Ui) {
        let height = ui.available_height();
        self.links_to_display = filter_links(&self.mirror_links, &self.filter);
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
