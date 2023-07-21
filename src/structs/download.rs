use std::collections::HashSet;
use std::thread;
use std::time::Instant;

use crossbeam_channel::Receiver;
use eframe::egui::{Button, Checkbox, Label, ScrollArea, Sense, TextEdit, Ui};
use tokio::runtime::Runtime;

use crate::functions::download::{fix_multiup_links, generate_direct_links};
use crate::functions::filter::{filter_links, set_filter_hosts};
use crate::structs::filter::FilterMenu;
use crate::structs::hosts::{DirectLink, LinkInformation, MirrorLink};

#[derive(Default)]
struct Receivers {
    direct_links: Option<Receiver<(usize, MirrorLink)>>,
    generating: Option<Receiver<bool>>,
}

impl Receivers {
    fn new(
        direct_links_receiver: Option<Receiver<(usize, MirrorLink)>>,
        generating_receiver: Option<Receiver<bool>>,
    ) -> Self {
        Self {
            direct_links: direct_links_receiver,
            generating: generating_receiver,
        }
    }
}

#[derive(Default)]
pub struct Download {
    multiup_links: String,
    mirror_links: Vec<(usize, MirrorLink, bool)>,
    recheck_status: bool,
    total_number_of_links: usize,
    number_of_processed_links: usize,
    generating: bool,
    cancelled: bool,
    timer: Option<Instant>,
    time_elapsed: u128,
    direct_links: Vec<DirectLink>,
    selection_indices: (Option<usize>, Option<usize>),
    info_indices: (Option<usize>, Option<usize>),
    selected_links: Vec<String>,
    receivers: Receivers,
    filter_menu: FilterMenu,
}

impl Download {
    pub fn show(ui: &mut Ui, download: &mut Download) {
        download.input_links_ui(ui);
        download.link_generation_ui(ui);
        download.display_links_ui(ui);
    }

    fn input_links_ui(&mut self, ui: &mut Ui) {
        let height = ui.available_height() / 2.0;
        ui.heading("MultiUp Links:");

        ui.vertical(|ui| {
            ui.set_max_height(height); // Sets the input portion to half of the window
            let height = ui.available_height() / 2.0; // A quarter of the window
            ScrollArea::vertical()
                .id_source("Link Input Box")
                .max_height(height)
                .min_scrolled_height(height)
                .show(ui, |ui| {
                    ui.add(TextEdit::multiline(&mut self.multiup_links)
                        .hint_text("Enter your Multiup links separated by a new line\n\
                        Supports short and long links, as well as older ones!")
                        .desired_width(ui.available_width())
                    )
                });

            let height = ui.available_height() / 2.0; // Remaining height after input box to fill a quarter the window

            let mut link_information: Vec<(Option<LinkInformation>, &mut bool)> = self.mirror_links.iter_mut()
                .map(|(_order, mirror_link, selected)| (mirror_link.information.clone(), selected))
                .collect();

            let mut selection = -1;
            if !link_information.is_empty() && link_information[0].0.is_some() {
                ui.collapsing("Link Information", |ui| {
                    ScrollArea::vertical()
                        .id_source("Link Information")
                        .min_scrolled_height(height)
                        .show(ui, |ui| {
                            for i in 0..link_information.len() {
                                let file = link_information[i].0.clone().unwrap();
                                ui.horizontal(|ui| {
                                    let selected = &mut link_information[i].1;
                                    let checkbox = ui.add(Checkbox::new(selected, ""));
                                    let shift_is_down = ui.ctx().input(|ui| ui.modifiers.shift);
                                    if shift_is_down && checkbox.clicked() {
                                        if self.info_indices.0.is_none() {
                                            self.info_indices.0 = Some(i);
                                        } else {
                                            self.info_indices.1 = Some(i);
                                        }
                                    } else if checkbox.clicked() {
                                        self.info_indices.0 = Some(i);
                                    }
                                    checkbox.context_menu(|ui| {
                                        if ui.button("Select all").clicked() {
                                            selection = 1;
                                            ui.close_menu();
                                        } else if ui.button("Deselect all").clicked() {
                                            selection = 0;
                                            ui.close_menu();
                                        }
                                    });
                                    ui.label({
                                        match &file.description {
                                            Some(description) => {
                                                format!("{} | {} ({} bytes). Uploaded {} ({} seconds). Total downloads: {}",
                                                        file.file_name,
                                                        description,
                                                        file.size,
                                                        file.date_upload,
                                                        file.time_upload,
                                                        file.number_downloads,
                                                )
                                            }
                                            None => {
                                                format!("{} ({} bytes). Uploaded {} ({} seconds). Total downloads: {}",
                                                        file.file_name,
                                                        file.size,
                                                        file.date_upload,
                                                        file.time_upload,
                                                        file.number_downloads,
                                                )
                                            }
                                        }
                                    });
                                });
                            }
                        });
                });
            };

            if selection == 0 {
                for i in self.mirror_links.iter_mut() {
                    i.2 = false;
                }
            } else if selection == 1 {
                for i in self.mirror_links.iter_mut() {
                    i.2 = true;
                }
            }
            if self.info_indices.0.is_some() && self.info_indices.1.is_some() {
                if self.info_indices.0.unwrap() > self.info_indices.1.unwrap() {
                    (self.info_indices.0, self.info_indices.1) = (self.info_indices.1, self.info_indices.0);
                }
                for (i, j) in self.mirror_links.iter_mut().enumerate() {
                    if i >= self.info_indices.0.unwrap() && i <= self.info_indices.1.unwrap() {
                        j.2 = true;
                    }
                };
                self.info_indices.0.take();
                self.info_indices.1.take();
            }
        });
    }

    fn link_generation_ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.recheck_status, "Re-check host status");

            if ui
                .add_enabled(!self.generating, Button::new("Generate links"))
                .clicked()
            {
                let (direct_links_tx, direct_links_rx) = crossbeam_channel::unbounded();
                let (generating_tx, generating_rx) = crossbeam_channel::unbounded();

                self.receivers = Receivers::new(
                    Some(direct_links_rx),
                    Some(generating_rx),
                );
                self.total_number_of_links = 0;
                self.number_of_processed_links = 0;
                self.mirror_links = vec![];
                self.timer = Some(Instant::now());
                self.generating = true;
                self.cancelled = false;
                let rt = Runtime::new().unwrap();

                let recheck_status = self.recheck_status;
                let multiup_links = self.multiup_links.clone();
                let mut mirror_links = fix_multiup_links(multiup_links);
                self.total_number_of_links = mirror_links.len();
                thread::spawn(move || {
                    rt.block_on(async {
                        generate_direct_links(
                            &mut mirror_links,
                            recheck_status,
                            direct_links_tx,
                        ).await;
                    });
                    let _ = generating_tx.send(false);
                });
            };

            if self.generating && !self.cancelled {
                ui.spinner();
                ui.label("Generating...");
                if ui.button("Cancel").clicked() {
                    self.cancelled = true;
                    self.generating = false;
                }
            } else if self.cancelled {
                ui.label("Cancelled!");
            } else if self.total_number_of_links > 0 {
                ui.label("Generated!");
            };

            if !self.cancelled {
                if let Some(timer) = self.timer {
                    self.time_elapsed = timer.elapsed().as_millis();
                };

                if self.time_elapsed != 0 && self.total_number_of_links > 0 {
                    let formatted_time = format!(
                        "Time taken: {}.{}s",
                        self.time_elapsed / 1000,
                        self.time_elapsed % 1000
                    );
                    ui.label(formatted_time);
                    let formatted_progress = format!(
                        "{}/{} completed.",
                        self.number_of_processed_links, self.total_number_of_links
                    );
                    ui.label(formatted_progress);
                }

                if let Some(rx) = &self.receivers.direct_links {
                    while let Ok((order, mirror_link)) = rx.try_recv() {
                        let index = self.mirror_links.binary_search_by_key(&order, |&(o, _, _)| o).unwrap_or_else(|x| x);
                        self.mirror_links.insert(index, (order, mirror_link, true));

                        self.number_of_processed_links = self.mirror_links.len();
                        let mut link_information = vec![];
                        for (_, mirror_link, _) in self.mirror_links.iter() {
                            if let Some(information) = mirror_link.information.clone() {
                                link_information.push(information);
                            };
                        }

                        let direct_links: Vec<DirectLink> = self.mirror_links.iter().filter_map(|(_, mirror_link, _)| mirror_link.direct_links.clone()).flatten().collect();
                        self.direct_links = direct_links;
                        self.filter_menu.hosts = set_filter_hosts(&self.direct_links);
                    };
                };

                if let Some(rx) = &self.receivers.generating {
                    if let Ok(generating) = rx.try_recv() {
                        self.generating = generating;
                        if !self.generating {
                            self.timer.take();
                        };
                    };
                };
            }
        });
    }

    fn display_links_ui(&mut self, ui: &mut Ui) {
        let direct_links: Vec<DirectLink> = self.mirror_links.iter_mut().filter_map(|(_order, mirror_link, displayed)| {
            if let Some(direct_link) = mirror_link.direct_links.clone() {
                let mut direct_links = vec![];
                for mut link in direct_link.iter().cloned() {
                    link.displayed = *displayed;
                    direct_links.push(link);
                }
                Some(direct_links)
            } else {
                None
            }
        }).flatten().collect();

        self.direct_links = direct_links;
        //self.display_links = filter_links(&self.direct_links, &self.filter_menu);
        let display_links = filter_links(&self.direct_links, &self.filter_menu);
        let height = ui.available_height();
        ui.horizontal(|ui| {
            ui.set_height(height);
            ui.vertical(|ui| {
                ui.heading("Direct Links:");
                ScrollArea::vertical().min_scrolled_height(ui.available_height()).id_source("Direct Link Output Box").show(ui, |ui| {
                    ui.set_width(ui.available_width() - 200.0);

                    ui.vertical(|ui| {
                        let mut selected_links: HashSet<&str> = self.selected_links.iter().map(|url| url.as_str()).collect();
                        // If a selected link is being filtered out, it will be unselected
                        for link in self.selected_links.iter() {
                            if !display_links.iter().any(|(_, l)| l.as_str() == link) {
                                selected_links.remove(link.as_str());
                            };
                        };

                        let (control_is_down, shift_is_down) = ui.ctx().input(|ui| (ui.modifiers.ctrl, ui.modifiers.shift));

                        for (_, link) in display_links.iter() {
                            let link_label = ui.add(Label::new(link).sense(Sense::click()));
                            if link_label.hovered() || self.selected_links.contains(link) {
                                link_label.clone().highlight();
                            };

                            if link_label.clicked() {
                                if control_is_down {
                                    if !selected_links.remove(link.as_str()) {
                                        selected_links.insert(link);
                                    };
                                    self.selection_indices = (None, None);
                                } else if shift_is_down {
                                    if self.selection_indices.0.is_none() {
                                        self.selection_indices.0 = Some(display_links.iter().position(|(_, url)| url == link).unwrap());
                                    } else {
                                        self.selection_indices.1 = Some(display_links.iter().position(|(_, url)| url == link).unwrap());
                                    };
                                } else {
                                    self.selection_indices.0 = Some(display_links.iter().position(|(_, url)| url == link).unwrap())
                                };
                            };

                            if self.selection_indices.1.is_some() && self.selection_indices.0 > self.selection_indices.1 {
                                (self.selection_indices.0, self.selection_indices.1) = (self.selection_indices.1, self.selection_indices.0)
                            };

                            if let (Some(index_1), Some(index_2)) = self.selection_indices {
                                display_links[index_1..=index_2].iter().for_each(|(_, link)| { selected_links.insert(link); });
                                if ui.ctx().input(|ui| !ui.modifiers.shift) {
                                    self.selection_indices = (None, None);
                                };
                            };

                            link_label.context_menu(|ui| {
                                if ui.button("Copy link").clicked() {
                                    ui.output_mut(|output| output.copied_text = link.to_string());
                                    ui.close_menu();
                                };

                                if !self.selected_links.is_empty() && ui.button("Copy selected links").clicked() {
                                    ui.output_mut(|output| output.copied_text = self.selected_links.join("\n"));
                                    ui.close_menu();
                                };

                                if ui.button("Copy all links").clicked() {
                                    let urls = display_links.iter().map(|(_, url)| url.clone()).collect::<Vec<String>>();
                                    ui.output_mut(|output| output.copied_text = urls.join("\n"));
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
                                    for (_, link) in display_links.iter() {
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
            FilterMenu::show(ui, &mut self.filter_menu);
        });
    }
}

#[derive(Default)]
pub struct ParsedTitle {
    pub file_name: String,
    pub size: f64,
    pub unit: String,
}

impl ParsedTitle {
    pub fn new(file_name: String, size: f64, unit: String) -> Self {
        ParsedTitle {
            file_name,
            size,
            unit,
        }
    }
}