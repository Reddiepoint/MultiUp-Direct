use std::collections::HashSet;
use std::sync::{Arc, mpsc, Mutex};
use std::time::Instant;
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use eframe::egui::{Button, Label, ScrollArea, Sense, TextEdit, Ui};
use eframe::egui::accesskit::AriaCurrent::False;
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
    pub receiver: Option<Receiver<(Vec<Link>, Vec<(String, bool)>)>>,
    pub timer_start: Option<Instant>,
    pub generation_time_receiver: Option<Receiver<u128>>,
    pub generation_time: u128,
    pub generation_status_receiver: Option<Receiver<bool>>,
    pub generation_status: Option<bool>,
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
            receiver: None,
            timer_start: None,
            generation_time_receiver: None,
            generation_time: 0,
            generation_status_receiver: None,
            generation_status: None,
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

    //fn generate_links_ui(&mut self, ui: &mut Ui) {
    //    ui.horizontal(|ui| {
    //        // Check status
    //        ui.checkbox(&mut self.check_status, "Re-check host status");
    //
    //        let button = ui.add(Button::new("Generate links"));
    //        // Generate links
    //        if button.clicked() {
    //            // Create runtime
    //            let mirror_links = self.mirror_links_input.clone();
    //            let check_status = self.check_status;
    //            let rt = Runtime::new().expect("Unable to create runtime");
    //            let _ = rt.enter();
    //
    //            // Create channels for communication
    //            let (link_tx, link_rx) = crossbeam_channel::unbounded();
    //            let (status_tx, status_rx) = crossbeam_channel::unbounded();
    //            let status_rx_clone = status_rx.clone();
    //            let (time_tx, time_rx) = crossbeam_channel::unbounded();
    //            let (timer_tx, timer_rx) = crossbeam_channel::unbounded();
    //
    //            // Store the receivers in fields to use later
    //            self.receiver = Some(link_rx);
    //            self.generation_time_receiver = Some(time_rx);
    //            self.generation_status_receiver = Some(status_rx);
    //
    //
    //            let time_elapsed: u128 = 0;
    //            let generating = true;
    //            // Send initial values
    //            time_tx.send(time_elapsed);
    //            status_tx.send(generating);
    //            timer_tx.send(true);
    //
    //            // Start timer
    //            // Timer
    //            std::thread::spawn(move || {
    //                let start_time = std::time::Instant::now();
    //                loop {
    //                    // Check if the generation is done
    //                    if let Ok(timer) = timer_rx.try_recv() {
    //                        if !timer {
    //                            break;
    //                        }
    //                    };
    //
    //                    let elapsed_time = start_time.elapsed().as_millis();
    //                    time_tx.send(elapsed_time).expect("Unable to send time");
    //                    std::thread::sleep(std::time::Duration::from_millis(100));
    //                }
    //            });
    //
    //            // Spawn a thread to generate direct links
    //            std::thread::spawn(move || {
    //                let result = rt.block_on(async {
    //                    generate_direct_links(&mirror_links, check_status).await
    //                });
    //                link_tx.send(result).expect("Unable to send result");
    //                status_tx.send(false);
    //                timer_tx.send(false);
    //
    //            });
    //
    //
    //
    //
    //        }
    //
    //        // Show generation status and time
    //        if let Some(generating) = self.generation_status {
    //            if generating {
    //                ui.spinner();
    //                ui.label("Generating");
    //            } else {
    //                ui.label("Generated");
    //            };
    //        };
    //
    //        let formatted_time = format!("Time elapsed: {}.{}s", self.generation_time / 1000, self.generation_time % 1000);
    //        ui.label(formatted_time);
    //
    //        // Update fields from receivers
    //        if let Some(rx) = &self.generation_time_receiver {
    //            if let Ok(time) = rx.try_recv() {
    //                self.generation_time = time;
    //            }
    //        }
    //
    //        if let Some(rx) = &self.generation_status_receiver {
    //            if let Ok(generating) = rx.try_recv() {
    //                self.generation_status = Some(generating);
    //            }
    //        }
    //        if let Some(rx) = &self.receiver {
    //            if let Ok((generated_links, links_hosts)) = rx.try_recv() {
    //                self.direct_links = generated_links;
    //                self.filter.hosts = links_hosts;
    //                self.receiver = None;
    //            }
    //        }
    //    });
    //}

    fn generate_links_ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Check status
            ui.checkbox(&mut self.check_status, "Re-check host status");
            let button;
            if let Some(status) = self.generation_status {
                if status {
                    button = ui.add_enabled(false, Button::new("Generate links"));
                } else {
                    button = ui.add(Button::new("Generate links"));
                }
            } else {
                button = ui.add(Button::new("Generate links"));
            }

            // Generate links
            if button.clicked() {
                // Create runtime
                let mirror_links = self.mirror_links_input.clone();
                let check_status = self.check_status;
                let rt = Runtime::new().expect("Unable to create runtime");
                let _ = rt.enter();

                // Create channels for communication
                let (link_tx, link_rx) = crossbeam_channel::unbounded();
                let (status_tx, status_rx) = crossbeam_channel::unbounded();
                let (timer_tx, timer_rx) = (status_tx.clone(), status_rx.clone());
                let (time_tx, time_rx) = crossbeam_channel::unbounded();
                let timer_start = timer_rx.clone();

                // Store the receivers in fields to use later
                self.receiver = Some(link_rx);
                self.generation_time_receiver = Some(time_rx);
                self.generation_status_receiver = Some(status_rx);


                // Send initial values
                time_tx.send(0);
                status_tx.send(true);
                timer_tx.send(true);


                self.timer_start = Some(std::time::Instant::now());
                // Spawn a thread to generate direct links
                std::thread::spawn(move || {
                    let result = rt.block_on(async {
                        generate_direct_links(&mirror_links, check_status).await
                    });
                    link_tx.send(result).expect("Unable to send result");
                    status_tx.send(false);
                    timer_tx.send(false);
                });


                // Start timer
                //let timer_start = self.timer_start.clone();

                //std::thread::spawn(move || {
                //    let start_time = std::time::Instant::now();
                //    loop {
                //        //if let Ok(timer) = timer_start.clone().unwrap().try_recv() {
                //        //    println!("Timer start: {}", timer);
                //        //    if !timer {
                //        //        break;
                //        //    }
                //        //};
                //        match timer_start.try_recv() {
                //            Ok(timer) => {
                //                println!("Timer on: {}", timer);
                //                if !timer {
                //                    break;
                //                }
                //            },
                //            Err(error) => {
                //                //panic!("Error: {}", error)
                //            }
                //        };
                //        let elapsed_time = start_time.elapsed().as_millis();
                //        time_tx.send(elapsed_time).expect("Unable to send time");
                //        std::thread::sleep(std::time::Duration::from_millis(100));
                //
                //
                //    }
                //});
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

            let formatted_time = format!("Time elapsed: {}.{}s", self.generation_time / 1000, self.generation_time % 1000);
            ui.label(formatted_time);

            // Update fields from receivers

            if let Some(rx) = &self.generation_status_receiver {
                if let Ok(generating) = rx.try_recv() {
                    self.generation_status = Some(generating);
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


            if let Some(rx) = &self.receiver {
                if let Ok((generated_links, links_hosts)) = rx.try_recv() {
                    self.direct_links = generated_links;
                    self.filter.hosts = links_hosts;
                    self.receiver = None;
                }
            }
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
