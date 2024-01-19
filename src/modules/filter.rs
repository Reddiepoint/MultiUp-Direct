use std::collections::BTreeSet;
use eframe::egui::{ScrollArea, Ui};
use crate::modules::links::{DownloadLink, MultiUpLink};

#[derive(Debug)]
pub struct FilterMenu {
    pub valid: bool,
    pub invalid: bool,
    pub unknown: bool,
    pub unchecked: bool,
    pub hosts: Vec<(String, bool)>,
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
    pub fn show(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            // ui.set_max_width(200.0);
            ui.label("Host validity: ");
            ui.checkbox(&mut self.valid, "Valid");
            ui.checkbox(&mut self.invalid, "Invalid");
            ui.checkbox(&mut self.unknown, "Unknown");
            ui.checkbox(&mut self.unchecked, "Unchecked");

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
                    let checkbox = ui.checkbox(&mut host.1, host_name);
                    checkbox.context_menu(|ui| {
                        if ui.button(format!("Select {} links only", host_name)).clicked() {
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
        let mut hosts: BTreeSet<String> = BTreeSet::new();
        for link in links {
            match link {
                MultiUpLink::Project(project) => {
                    if let Some(Ok(_)) = &project.status {
                        for link in project.download_links.as_ref().unwrap() {
                            for link in link.direct_links.as_ref().unwrap() {
                                hosts.insert(link.host.to_string());
                            }
                        }
                    }
                }
                MultiUpLink::Download(download) => {
                    if let Some(Ok(_)) = &download.status {
                        for link in download.direct_links.as_ref().unwrap() {
                            hosts.insert(link.host.to_string());
                        }
                    }
                }
            }
        }
        self.hosts = hosts.into_iter().map(|host| (host, true)).collect();
    }
    pub fn filter_links(&self, download_link: &DownloadLink) -> Vec<String> {
        let displayed_links: Vec<String> = vec![];
        return match &download_link.direct_links {
            None => displayed_links,
            Some(links) => {
                links.iter()
                    .filter(|link| {
                        let host_check = self.hosts.iter().any(|(host_name, checked)| {
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