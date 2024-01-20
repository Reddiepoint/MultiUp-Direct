use std::collections::BTreeMap;
use eframe::egui::{ScrollArea, Ui};
use crate::modules::links::{DownloadLink, MultiUpLink};

#[derive(Debug)]
pub struct FilterMenu {
    pub valid: bool,
    pub invalid: bool,
    pub unknown: bool,
    pub unchecked: bool,
    pub hosts: Vec<(String, bool, u32)>,
}

impl Default for FilterMenu {
    fn default() -> Self {
        FilterMenu {
            valid: true,
            invalid: false,
            unknown: false,
            unchecked: false,
            hosts: vec![],
        }
    }
}

impl FilterMenu {
    pub fn show(&mut self, ui: &mut Ui, links: &Vec<MultiUpLink>) {
        // Calculate number of each type
        let mut valid: u32 = 0;
        let mut invalid: u32 = 0;
        let mut unknown: u32 = 0;
        let mut unchecked: u32 = 0;

        for link in links {
            match link {
                MultiUpLink::Project(project) => {
                    if let Some(Ok(_)) = &project.status {
                        for download_link in project.download_links.as_ref().unwrap() {
                            if let Some(Ok(())) = &download_link.status {
                                for link in download_link.direct_links.as_ref().unwrap() {
                                    match link.validity.as_str() {
                                        "valid" => valid += 1,
                                        "invalid" => invalid += 1,
                                        "unknown" => unknown += 1,
                                        _ => unchecked += 1,
                                    }
                                }
                            }
                        }
                    }
                }
                MultiUpLink::Download(download) => {
                    if let Some(Ok(())) = &download.status {
                        for link in download.direct_links.as_ref().unwrap() {
                            match link.validity.as_str() {
                                "valid" => valid += 1,
                                "invalid" => invalid += 1,
                                "unknown" => unknown += 1,
                                _ => unchecked += 1,
                            }
                        }
                    }

                }
            }
        }
        ui.vertical(|ui| {
            // ui.set_max_width(200.0);

            ui.label("Host validity: ");
            ui.checkbox(&mut self.valid, format!("Valid ({})", valid));
            ui.checkbox(&mut self.invalid, format!("Invalid ({})", invalid));
            ui.checkbox(&mut self.unknown, format!("Unknown ({})", unknown));
            ui.checkbox(&mut self.unchecked, format!("Unchecked ({})", unchecked));

            ui.separator();

            ui.label("Show links for hosts: ");
            if ui.button("Select all").clicked() {
                for host in self.hosts.iter_mut() {
                    if !host.1 {
                        host.1 = true
                    }
                }
            };
            if ui.button("Deselect all").clicked() {
                for host in self.hosts.iter_mut() {
                    if host.1 {
                        host.1 = false
                    }
                }
            };

            ui.separator();

            ScrollArea::vertical().id_source("Host Filter").min_scrolled_height(ui.available_height()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                for i in 0..self.hosts.len() {
                    let host = &mut self.hosts[i];
                    let host_name = &host.0.clone();
                    let checkbox = ui.checkbox(&mut host.1, format!("{} ({})", host_name, host.2));
                    checkbox.context_menu(|ui| {
                        if ui.button(format!("Show {} links only", host_name)).clicked() {
                            for host in self.hosts.iter_mut() {
                                host.1 = &host.0 == host_name;
                            };
                            ui.close_menu();
                        }
                    });
                }
            });
        });
    }

    pub fn update_hosts(&mut self, links: &Vec<MultiUpLink>) {
        let mut hosts: BTreeMap<String, u32> = BTreeMap::new();
        for link in links {
            match link {
                MultiUpLink::Project(project) => {
                    if let Some(Ok(_)) = &project.status {
                        for link in project.download_links.as_ref().unwrap() {
                            if let Some(Ok(_)) = &link.status {
                                for link in link.direct_links.as_ref().unwrap() {
                                    hosts.entry(link.host.to_string()).and_modify(|count| *count += 1).or_insert(1);
                                }
                            }

                        }
                    }
                }
                MultiUpLink::Download(download) => {
                    if let Some(Ok(_)) = &download.status {
                        for link in download.direct_links.as_ref().unwrap() {
                            hosts.entry(link.host.to_string()).and_modify(|count| *count += 1).or_insert(1);
                        }
                    }
                }
            }
        }
        self.hosts = hosts.into_iter().map(|(host, count)| (host, true, count)).collect();
    }
    pub fn filter_links(&self, download_link: &DownloadLink) -> Vec<String> {
        let displayed_links: Vec<String> = vec![];
        return match &download_link.direct_links {
            None => displayed_links,
            Some(links) => {
                links.iter()
                    .filter(|link| {
                        let host_check = self.hosts.iter().any(|(host_name, checked, _count)| {
                           &link.host == host_name && *checked
                        });

                        let validity_match = match link.validity.as_str() {
                            "valid" => self.valid,
                            "invalid" => self.invalid,
                            "unknown" => self.unknown,
                            _ => self.unchecked,
                        };

                        host_check && validity_match
                    })
                    .map(|link| link.url.clone())
                    .collect()
            }
        }
    }
}