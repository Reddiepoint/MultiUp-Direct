use std::collections::BTreeSet;
use std::collections::HashSet;
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use async_recursion::async_recursion;
use crossbeam_channel::{Sender, TryRecvError};
use crossbeam_channel::Receiver;
use eframe::egui::{Button, Checkbox, Label, ScrollArea, Sense, TextEdit, Ui};
use reqwest::{Client, StatusCode};
use scraper::{Element, Selector};
use tokio::runtime::Runtime;

use crate::modules::filter::{filter_links, FilterMenu, set_filter_hosts};
use crate::modules::links::{check_validity, DirectLink, LinkInformation, MirrorLink};

#[derive(Default)]
struct Receivers {
    direct_links: Option<Receiver<(usize, MirrorLink)>>,
    generating: Option<Receiver<bool>>,
    total_links: Option<Receiver<usize>>,
}

impl Receivers {
    fn new(
        direct_links_receiver: Option<Receiver<(usize, MirrorLink)>>,
        generating_receiver: Option<Receiver<bool>>,
        total_links_receiver: Option<Receiver<usize>>,
    ) -> Self {
        Self {
            direct_links: direct_links_receiver,
            generating: generating_receiver,
            total_links: total_links_receiver,
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
    display_links: Vec<(bool, String)>,
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
            ScrollArea::vertical().id_source("Link Input Box").max_height(height).min_scrolled_height(height).min_scrolled_width(ui.available_width()).show(ui, |ui| {
                ui.add(TextEdit::multiline(&mut self.multiup_links).hint_text("Enter your Multiup links separated by a new line\n\
                        Supports short and long links, as well as older ones!").desired_width(ui.available_width())
                )
            });

            let height = ui.available_height() / 2.0; // Remaining height after input box to fill a quarter the window

            let mut link_information: Vec<(Option<LinkInformation>, &mut bool)> = self.mirror_links.iter_mut().map(|(_order, mirror_link, selected)| (mirror_link.information.clone(), selected)).collect();

            let mut selection = -1;
            if !link_information.is_empty() && link_information[0].0.is_some() {
                ui.collapsing("Link Information", |ui| {
                    ScrollArea::vertical().id_source("Link Information").min_scrolled_height(height).min_scrolled_width(ui.available_width()).show(ui, |ui| {
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
                                    let description = file.description.as_ref().map_or(String::new(), |desc| format!(" | {}", desc));
                                    format!("{}{} ({} bytes). Uploaded {} ({} seconds). Total downloads: {}",
                                            file.file_name,
                                            description,
                                            file.size,
                                            file.date_upload,
                                            file.time_upload,
                                            file.number_downloads,
                                    )
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

            if ui.add_enabled(!self.generating, Button::new("Generate links")).clicked() {
                let (direct_links_tx, direct_links_rx) = crossbeam_channel::bounded(200);
                let (generating_tx, generating_rx) = crossbeam_channel::unbounded();
                let (total_links_tx, total_links_rx) = crossbeam_channel::unbounded();
                self.receivers = Receivers::new(
                    Some(direct_links_rx),
                    Some(generating_rx),
                    Some(total_links_rx),
                );

                let recheck_status = self.recheck_status;
                let multiup_links = self.multiup_links.clone();
                let rt = Runtime::new().unwrap();
                self.timer = Some(Instant::now());

                thread::spawn(move || {
                    rt.block_on(async {
                        let mut mirror_links = fix_multiup_links(multiup_links);
                        let length = mirror_links.len();
                        if length == 0 {
                            return;
                        }
                        let _ = total_links_tx.send(mirror_links.len());
                        generate_direct_links(
                            &mut mirror_links,
                            recheck_status,
                            direct_links_tx,
                        ).await;
                    });
                    let now = Instant::now();
                    let _ = generating_tx.send(false);
                    let after = Instant::now();
                    println!("Time to fix: {}", (after - now).as_micros());
                });

                self.total_number_of_links = 0;
                self.number_of_processed_links = 0;
                self.mirror_links = vec![];
                self.generating = true;
                self.cancelled = false;
            };

            self.update_generation_status();

            self.update_total_number_of_links();

            self.update_direct_links();

            self.display_time_and_progress(ui);
        });
    }

    fn display_links_ui(&mut self, ui: &mut Ui) {
        let height = ui.available_height();
        ui.horizontal(|ui| {
            ui.set_height(height);
            ui.vertical(|ui| {
                ui.heading("Direct Links:");
                ScrollArea::vertical().min_scrolled_height(ui.available_height()).min_scrolled_width(ui.available_width() - 200.0).id_source("Direct Link Output Box").show(ui, |ui| {
                    ui.set_width(ui.available_width() - 200.0);
                    ui.vertical(|ui| {
                        if self.display_links.is_empty() {
                            return;
                        }
                        let selected_links: HashSet<&String> = self.selected_links.iter().collect();
                        let displayed_links: HashSet<&String> = self.display_links.iter().map(|(_, url)| url).collect();
                        let mut selected_links: HashSet<&str> = selected_links.intersection(&displayed_links).map(|link| link.as_str()).collect();

                        let (control_is_down, shift_is_down) = ui.ctx().input(|ui| (ui.modifiers.ctrl, ui.modifiers.shift));

                        let now = Instant::now();

                        for (_, link) in self.display_links.iter() {
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
                                        self.selection_indices.0 = Some(self.display_links.iter().position(|(_, url)| url == link).unwrap());
                                    } else {
                                        self.selection_indices.1 = Some(self.display_links.iter().position(|(_, url)| url == link).unwrap());
                                    };
                                } else {
                                    self.selection_indices.0 = Some(self.display_links.iter().position(|(_, url)| url == link).unwrap())
                                };
                            };

                            if self.selection_indices.1.is_some() && self.selection_indices.0 > self.selection_indices.1 {
                                (self.selection_indices.0, self.selection_indices.1) = (self.selection_indices.1, self.selection_indices.0)
                            };

                            if let (Some(index_1), Some(index_2)) = self.selection_indices {
                                self.display_links[index_1..=index_2].iter().for_each(|(_, link)| { selected_links.insert(link); });
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
                                    let urls = self.display_links.iter().map(|(_, url)| url.clone()).collect::<Vec<String>>();
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
                                    for (_, link) in self.display_links.iter() {
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
                        let after = Instant::now();
                        println!("Time taken: {}", (after - now).as_micros());
                    });
                });
            });
            FilterMenu::show(ui, &mut self.filter_menu);
        });
    }

    fn update_generation_status(&mut self) {
        if let Some(rx) = &self.receivers.generating {
            if let Ok(generating) = rx.try_recv() {
                self.generating = generating;
            };
        };
    }

    fn update_total_number_of_links(&mut self) {
        if let Some(rx) = &self.receivers.total_links {
            if let Ok(number) = rx.try_recv() {
                self.total_number_of_links = number;
            }
        }
    }

    fn update_direct_links(&mut self) {
        if let Some(rx) = &self.receivers.direct_links {
            while let Ok((order, mirror_link)) = rx.try_recv() {
                let index = self.mirror_links.binary_search_by_key(&order, |&(o, _, _)| o).unwrap_or_else(|x| x);
                self.mirror_links.insert(index, (order, mirror_link, true));
            }

            self.number_of_processed_links = self.mirror_links.len();


            if !self.generating {
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
                self.filter_menu.hosts = set_filter_hosts(&self.direct_links);
                self.display_links = filter_links(&self.direct_links, &self.filter_menu);

                self.receivers.direct_links.take();
                self.receivers.generating.take();
                self.receivers.total_links.take();
                self.timer.take();
            }
        };
    }

    fn display_time_and_progress(&mut self, ui: &mut Ui) {
        if let Some(timer) = self.timer {
            self.time_elapsed = timer.elapsed().as_millis();
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

        if self.time_elapsed != 0 { //&& self.total_number_of_links > 0           && (self.number_of_processed_links > 0 || self.total_number_of_links > 0)
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

static MULTIUP_REGEX: OnceLock<regex::Regex> = OnceLock::new();
static MIRROR_REGEX: OnceLock<regex::Regex> = OnceLock::new();
static PROJECT_REGEX: OnceLock<regex::Regex> = OnceLock::new();

/// Convert short and long links to the en/mirror page. Removes duplicates
pub fn fix_multiup_links(multiup_links: String) -> Vec<MirrorLink> {
    let mirror_prefix = "https://multiup.org/en/mirror/";
    let multiup_regex = MULTIUP_REGEX.get_or_init(|| regex::Regex::new(r#"^https?://(www\.)?multiup\.org/(en/)?(download/)?"#).unwrap());
    let mirror_regex = MIRROR_REGEX.get_or_init(|| regex::Regex::new(r#"^https?://multiup\.org/en/mirror/[^/]+/[^/]+$"#).unwrap());
    let project_regex = PROJECT_REGEX.get_or_init(|| regex::Regex::new(r#"^https:\/\/(www\.)?multiup\.org\/(en\/)?project\/.*$"#).unwrap());

    let mut mirror_links: Vec<String> = Vec::with_capacity(multiup_links.lines().count()); // Pre-allocate memory for the vector
    let (multiup_links_tx, multiup_links_rx) = crossbeam_channel::unbounded();
    for line in multiup_links.lines() {
        if !line.contains("multiup") {
            continue;
        }
        let multiup_link = line.trim().split(' ').next().unwrap().to_string();
        if mirror_regex.is_match(&multiup_link) {
            if !mirror_links.contains(&multiup_link) {
                mirror_links.push(multiup_link.to_string());
            }
        } else if project_regex.is_match(&multiup_link) {
            let rt = Runtime::new().unwrap();
            let multiup_links_tx = multiup_links_tx.clone();
            thread::spawn(move || {
                rt.block_on(async {
                    let multiup_links = match get_project_links(&multiup_link).await {
                        Some(project_links) => fix_multiup_links(project_links.clone()),
                        None => vec![MirrorLink::new(multiup_link.to_string())]
                    };
                    let _ = multiup_links_tx.send(multiup_links);
                });
            });
        } else if multiup_regex.is_match(&multiup_link) {
            let suffix = multiup_regex.replace(&multiup_link, "");
            let mut fixed_link = format!("{}{}", mirror_prefix, suffix);
            if mirror_regex.is_match(&fixed_link) {
                if !mirror_links.contains(&fixed_link) {
                    mirror_links.push(fixed_link);
                };
            } else {
                fixed_link.push_str("/a");
                if mirror_regex.is_match(&fixed_link) && !mirror_links.contains(&fixed_link) {
                    mirror_links.push(fixed_link);
                };
            }
        }
    };

    drop(multiup_links_tx);
    let mut mirror_links: Vec<MirrorLink> = mirror_links.iter().map(|link| MirrorLink::new(link.to_string())).collect();

    loop {
        match multiup_links_rx.try_recv() {
            Ok(links) => {
                for link in links {
                    if !mirror_links.contains(&link) {
                        mirror_links.push(link);
                    }
                }
            }
            Err(TryRecvError::Empty) => {
                thread::sleep(Duration::from_millis(100));
            }
            Err(TryRecvError::Disconnected) => {
                // Channel is disconnected
                break;
            }
        }
    }

    mirror_links
}

static PROJECT_LINKS_SELECTOR: OnceLock<Selector> = OnceLock::new();

pub async fn get_project_links(url: &str) -> Option<String> {
    let client = Client::new();
    let html = match get_html(url, &client).await {
        Ok(html) => html,
        Err(_) => return None
    };
    let project_links_selector = PROJECT_LINKS_SELECTOR.get_or_init(|| Selector::parse(r#"#textarea-links-long"#).unwrap());
    let html = scraper::Html::parse_document(&html);
    let links = match html.select(project_links_selector).next() {
        Some(links) => links,
        None => return None
    };
    Some(links.inner_html().to_string())
}

pub async fn generate_direct_links(mirror_links: &mut [MirrorLink], recheck_status: bool, direct_links_tx: Sender<(usize, MirrorLink)>) {
    let client = Client::new();
    let mut tasks = Vec::new();
    for (order, link) in mirror_links.iter().enumerate() {
        let direct_links_tx = direct_links_tx.clone();
        let mut mirror_link = link.clone();
        let client = client.clone();
        tasks.push(tokio::spawn(async move {
            let mirror_link = scrape_link(&mut mirror_link, recheck_status, &client).await;
            let _ = direct_links_tx.send((order, mirror_link));
        }));
    }

    let _ = futures::future::join_all(tasks).await;
}

#[test]
fn test() {
    let a = vec![DirectLink {
        name_host: "!!!error".to_string(),
        url: "a".to_string(),
        validity: "invalid".to_string(),
        displayed: false,
    }];
    if a.contains(&DirectLink {
        name_host: "!!!error".to_string(),
        url: "".to_string(),
        validity: "".to_string(),
        displayed: false,
    }) {
        println!("true");
    }
}

async fn scrape_link(mirror_link: &mut MirrorLink, check_status: bool, client: &Client) -> MirrorLink {
    let link_hosts = scrape_link_for_hosts(&mirror_link.url, client).await;
    if link_hosts.1.contains(&DirectLink {
        name_host: "!!!error".to_string(),
        url: "".to_string(),
        validity: "".to_string(),
        displayed: false,
    }) {
        let url = mirror_link.url.clone();
        let error = link_hosts.1.first().unwrap().url.clone();

        let url = match url.strip_suffix("/a") {
            Some(url) => url.to_string(),
            None => url
        };
        //println!("{error} - {url}");
        mirror_link.direct_links = Some(BTreeSet::from([DirectLink::new("!!!error".to_string(), format!("{} - {}", error, url), "invalid".to_string())]));
        mirror_link.information = Some(LinkInformation {
            error: "invalid".to_string(),
            file_name: error,
            size: 0.to_string(),
            date_upload: "".to_string(),
            time_upload: 0,
            date_last_download: "N/A".to_string(),
            number_downloads: 0,
            description: Some(url),
            hosts: Default::default(),
        });
        return std::mem::take(mirror_link);
    }
    if !check_status {
        //link_hosts.1.sort_by_key(|link| link.name_host.clone());
        mirror_link.direct_links = Some(link_hosts.1);
        let mut parsed_title = link_hosts.0;
        match parsed_title.unit.to_lowercase().as_str() {
            "kb" => parsed_title.size *= 1024.0,
            "mb" => parsed_title.size *= 1048576.0,
            "gb" => parsed_title.size *= 1073741824.0,
            _ => {}
        };
        parsed_title.size = parsed_title.size.floor();
        mirror_link.information = Some(LinkInformation {
            error: "success".to_string(),
            file_name: parsed_title.file_name,
            size: parsed_title.size.to_string(),
            date_upload: "".to_string(),
            time_upload: 0,
            date_last_download: "N/A".to_string(),
            number_downloads: 0,
            description: None,
            hosts: Default::default(),
        });
        return std::mem::take(mirror_link);
    }
    let link_information = check_validity(&mirror_link.url).await;
    let direct_links: BTreeSet<DirectLink> = link_hosts.1.into_iter().map(|link| {
        let status = match link_information.hosts.get(&link.name_host) {
            Some(validity) => validity.clone().unwrap(),
            None => "unknown".to_string()
        };
        DirectLink::new(link.name_host, link.url, status)
    }).collect();
    //direct_links.sort_by_key(|link| link.name_host.clone());
    mirror_link.direct_links = Some(direct_links);
    mirror_link.information = Some(link_information);
    std::mem::take(mirror_link)
}

static SELECTOR: OnceLock<Selector> = OnceLock::new();
static FILE_NAME_SELECTOR: OnceLock<Selector> = OnceLock::new();
static QUEUE_SELECTOR: OnceLock<Selector> = OnceLock::new();

#[async_recursion]
async fn scrape_link_for_hosts(url: &str, client: &Client) -> (ParsedTitle, BTreeSet<DirectLink>) {
    // Regular links
    let mut links: BTreeSet<DirectLink> = BTreeSet::new();
    // Scrape panel
    //let now = Instant::now();
    let html = match get_html(url, client).await {
        Ok(html) => html,
        Err(error) => {
            let error = match error {
                LinkError::ReqwestError(error) => error.to_string(),
                LinkError::InvalidError => "Invalid link".to_string()
            };
            return (ParsedTitle::default(), BTreeSet::from([DirectLink::new("!!!error".to_string(), error.to_string(), "invalid".to_string())]));
        }
    };
    //println!("{}", html);
    //let after = Instant::now();
    //println!("Time taken to load: {}", (after - now).as_millis());

    let selector = SELECTOR.get_or_init(|| Selector::parse(r#"a.host, button.host"#).unwrap()); //button[type="submit"]
    let file_name_selector = FILE_NAME_SELECTOR.get_or_init(|| Selector::parse(r#"body > section > div > section > header > h2 > a"#).unwrap());
    let queue_selector = QUEUE_SELECTOR.get_or_init(|| Selector::parse(r#"body > section > div > section > div.row > div > section > div > div > div:nth-child(2) > div > h4"#).unwrap());

    {
        let website_html = scraper::Html::parse_document(&html);
        for element in website_html.select(selector) {
            let element_value = element.value();
            let name_host = match element_value.attr("namehost") {
                Some(name_host) => {
                    if name_host == "UseNext" {
                        continue;
                    }
                    name_host
                }
                None => break,
            };
            let link = element_value.attr("link").unwrap();
            let validity = element_value.attr("validity").unwrap();
            links.insert(DirectLink::new(name_host.to_string(), link.to_string(), validity.to_string()));
        };

        if let Some(element) = website_html.select(queue_selector).next() {
            if links.is_empty() {
                if let Some(element) = element.next_sibling_element() {
                    let text = element.inner_html().replace(r#"<strong class="amount">"#, "").replace("</strong>", "").trim().to_string();
                    let queue_status = if text == "File not found on servers" {
                        "File not found on servers".to_string()
                    } else if text.contains("Uploading") {
                        "Uploading".to_string()
                    } else if text.parse::<u16>().is_ok() {
                        format!("In queue ({})", text)
                    } else {
                        text
                    };
                    links.insert(DirectLink::new("!!!error".to_string(), queue_status.to_string(), "invalid".to_string()));
                }
            }
        }
    }

    if links.is_empty() {
        links.insert(DirectLink::new("!!!error".to_string(), "No hosts found".to_string(), "invalid".to_string()));
        return scrape_link_for_hosts(url, client).await;
    }

    let website_html = scraper::Html::parse_document(&html);
    let mirror_title = website_html.select(file_name_selector).next().unwrap().next_sibling().unwrap().value().as_text().unwrap().to_string();
    let title_stuff = parse_title(&mirror_title);
    (title_stuff, links)
}

fn parse_title(input: &str) -> ParsedTitle {
    let input = input.trim();
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() >= 3 {
        let file_name = parts[3..parts.len() - 4].join(" ");
        let size = parts[parts.len() - 3].parse::<f64>().unwrap_or(0.0);
        let unit = parts[parts.len() - 2];
        ParsedTitle::new(file_name, size, unit.to_string())
    } else {
        ParsedTitle::new(String::new(), 0.0, String::new())
    }
}

#[derive(Debug)]
pub enum LinkError {
    ReqwestError(reqwest::Error),
    InvalidError,
}

#[async_recursion]
pub async fn get_html(url: &str, client: &Client) -> Result<String, LinkError> {
    let a = match client.get(url).send().await {
        Ok(response) => response,
        Err(error) => return Err(LinkError::ReqwestError(error))
    };
    match a.error_for_status() {
        Ok(res) => Ok(res.text().await.unwrap().to_string()),
        Err(error) => {
            if error.status().unwrap() != StatusCode::NOT_FOUND {
                let _ = tokio::time::sleep(Duration::from_millis(100)).await;
                //eprintln!("{error}");
                return get_html(url, client).await;
            }
            Err(LinkError::InvalidError)
        }
    }
}