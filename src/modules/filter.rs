use std::collections::BTreeSet;

use eframe::egui::{ScrollArea, Ui};

use crate::modules::links::DirectLink;

#[derive(Clone)]
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
    pub fn show(ui: &mut Ui, filter: &mut FilterMenu) {
        ui.vertical(|ui| {
            ui.set_max_width(200.0);
            ui.label("Host status: ");
            ui.checkbox(&mut filter.valid, "Valid");
            ui.checkbox(&mut filter.invalid, "Invalid");
            ui.checkbox(&mut filter.unknown, "Unknown");
            ui.checkbox(&mut filter.unchecked, "Unchecked");
            ui.separator();
            ui.label("Show hosts: ");
            if ui.button("Select all").clicked() {
                for host in filter.hosts.iter_mut() {
                    if !host.1 {
                        host.1 = true
                    }
                }
            };
            if ui.button("Deselect all").clicked() {
                for host in filter.hosts.iter_mut() {
                    if host.1 {
                        host.1 = false
                    }
                }
            };

            ui.separator();

            ScrollArea::vertical().id_source("Host Filter").min_scrolled_height(ui.available_height()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                for i in 0..filter.hosts.len() {
                    let host = &mut filter.hosts[i];
                    let host_name = &host.0.clone();
                    let checkbox = ui.checkbox(&mut host.1, host_name);
                    checkbox.context_menu(|ui| {
                        if ui.button(format!("Select {} links only", host_name)).clicked() {
                            for host in filter.hosts.iter_mut() {
                                host.1 = &host.0 == host_name;
                            };
                            ui.close_menu();
                        }
                    });
                }
            });
        });
    }
}

pub fn filter_links(links: &[DirectLink], filter: &FilterMenu) -> Vec<(bool, String)> {
    links.iter().filter(|link| link.displayed).filter(|link| match link.validity.as_str() {
        "valid" => filter.valid,
        "invalid" => filter.invalid,
        "unknown" => filter.unknown,
        _ => filter.unchecked,
    }).filter(|link| {
        filter.hosts.iter().any(|(host_name, selected)| *selected && &link.host == host_name)
    }).map(|link| {
        (link.displayed, link.url.to_string())
    }).collect()
}

pub fn set_filter_hosts(links: &[DirectLink]) -> Vec<(String, bool)> {
    let mut hosts: BTreeSet<String> = BTreeSet::new();
    for link in links {
        hosts.insert(link.host.to_string());
    }

    hosts.into_iter().map(|host| (host, true)).collect()
}