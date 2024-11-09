use crate::modules::hosting_services::filter::{count_hosts, get_info};
use crate::modules::hosting_services::links::{DirectLink, LinkInformation, LinkValidity};
use eframe::egui::{ScrollArea, Ui};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct FilterHost {
    name: String,
    checked: bool,
    count: u32,
}

impl FilterHost {
    pub fn new(name: String, checked: bool, count: u32) -> Self {
        FilterHost {
            name,
            checked,
            count,
        }
    }
}

#[derive(Debug)]
pub struct FilterMenu {
    pub valid_chosen: bool,
    pub invalid_chosen: bool,
    pub unknown_chosen: bool,
    pub valid_number: u32,
    pub invalid_number: u32,
    pub unknown_number: u32,
    pub current_hosts: Vec<FilterHost>,
    pub full_hosts: Vec<String>,
}

impl Default for FilterMenu {
    fn default() -> Self {
        FilterMenu {
            valid_chosen: true,
            invalid_chosen: false,
            unknown_chosen: false,
            valid_number: 0,
            invalid_number: 0,
            unknown_number: 0,
            current_hosts: vec![],
            full_hosts: vec![],
        }
    }
}

impl FilterMenu {
    pub fn update_from_link_information(&mut self, links: &Vec<LinkInformation>) {
        let (valid, invalid, unknown, hosts) = get_info(links);
        self.valid_number = valid;
        self.invalid_number = invalid;
        self.unknown_number = unknown;
        let host_names = hosts.keys().map(|keys| keys.to_string()).collect();
        let hosts: Vec<FilterHost> = hosts
            .iter()
            .map(|(host, count)| FilterHost::new(host.clone(), true, *count))
            .collect();

        self.current_hosts = hosts.clone();
        self.full_hosts = host_names;
    }

    pub fn update_from_link_list(&mut self, hosts: &Vec<String>) {
        let hosts_count = count_hosts(hosts);
        // println!("{:?}", hosts_count);
        let mut new_hosts: Vec<FilterHost> = vec![];
        for host in hosts_count.keys() {
            match self.current_hosts.iter().find(|filter_host| &filter_host.name == host) {
                Some(host) => {
                    new_hosts.push(FilterHost::new(
                        host.name.clone(),
                        host.checked,
                        *hosts_count.get(&host.name).unwrap(),
                    ))
                }
                None => {
                    new_hosts.push(FilterHost::new(
                        host.to_string(),
                        false,
                        *hosts_count.get(host).unwrap(),
                    ));
                }
            }
        }

        self.current_hosts = new_hosts
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.label("Link validity: ");
            ui.checkbox(&mut self.valid_chosen, format!("Online ({})", self.valid_number));
            ui.checkbox(
                &mut self.invalid_chosen,
                format!("Offline ({})", self.invalid_number),
            );
            ui.checkbox(
                &mut self.unknown_chosen,
                format!("Unknown ({})", self.unknown_number),
            );

            ui.separator();
            ui.label("Hosts:");
            if ui.button("Select all").clicked() {
                for host in self.current_hosts.iter_mut() {
                    host.checked = true;
                }
            }
            
            ScrollArea::vertical()
                .id_salt("Host Filter")
                .show(ui, |ui| {
                    for i in 0..self.current_hosts.len() {
                        let host = &mut self.current_hosts[i];
                        let checkbox = ui.checkbox(&mut host.checked, format!("{} ({})", &host.name, &host.count));
                        let host_name = &host.name.clone();
                        checkbox.context_menu(|ui| {
                            if ui.button(format!("Show {} links only", host_name)).clicked() {
                                for current_host in self.current_hosts.iter_mut() {
                                    current_host.checked = &current_host.name == host_name;
                                };
                                ui.close_menu();
                            }
                        });
                    }
                });
        });
    }

    pub fn filter_link(&self, link: &DirectLink, filter_host_checked: bool, filter_validity: bool) -> bool {
        let mut host_check = HashMap::new();
        for host in &self.current_hosts {
            host_check.insert(&host.name, host.checked);
        }

        for host in &self.full_hosts {
            if link.host != host.as_str() {
                continue;
            }

            if filter_host_checked && !host_check.get(&link.host).unwrap_or(&false) {
                return false;
            }

            if filter_validity {
                return match link.validity {
                    LinkValidity::Valid => self.valid_chosen,
                    LinkValidity::Invalid => self.invalid_chosen,
                    LinkValidity::Unknown => self.unknown_chosen,
                };
            }
        }

        false
    }
}

// impl FilterMenu {
//     pub fn show(&mut self, ui: &mut Ui, links: &Vec<MultiUpLink>) {
//         // Calculate number of each type
//         let mut valid: u32 = 0;
//         let mut invalid: u32 = 0;
//         let mut unknown: u32 = 0;
//         let mut unchecked: u32 = 0;
//
//         for link in links {
//             match link {
//                 MultiUpLink::Project(project) => {
//                     if let Some(Ok(_)) = &project.status {
//                         for download_link in project.download_links.as_ref().unwrap() {
//                             if let Some(Ok(())) = &download_link.status {
//                                 for link in download_link.direct_links.as_ref().unwrap() {
//                                     match link.validity.as_str() {
//                                         "valid" => valid += 1,
//                                         "invalid" => invalid += 1,
//                                         "unknown" => unknown += 1,
//                                         _ => unchecked += 1,
//                                     }
//                                 }
//                             }
//                         }
//                     }
//                 }
//                 MultiUpLink::Download(download) => {
//                     if let Some(Ok(())) = &download.status {
//                         for link in download.direct_links.as_ref().unwrap() {
//                             match link.validity.as_str() {
//                                 "valid" => valid += 1,
//                                 "invalid" => invalid += 1,
//                                 "unknown" => unknown += 1,
//                                 _ => unchecked += 1,
//                             }
//                         }
//                     }
//
//                 }
//             }
//         }
//         ui.vertical(|ui| {
//             // ui.set_max_width(200.0);
//
//             ui.label("Host validity: ");
//             ui.checkbox(&mut self.valid, format!("Valid ({})", valid));
//             ui.checkbox(&mut self.invalid, format!("Invalid ({})", invalid));
//             ui.checkbox(&mut self.unknown, format!("Unknown ({})", unknown));
//             ui.checkbox(&mut self.unchecked, format!("Unchecked ({})", unchecked));
//
//             ui.separator();
//
//             ui.label("Show links for hosts: ");
//             if ui.button("Select all").clicked() {
//                 for host in self.hosts.iter_mut() {
//                     if !host.1 {
//                         host.1 = true
//                     }
//                 }
//             };
//             if ui.button("Deselect all").clicked() {
//                 for host in self.hosts.iter_mut() {
//                     if host.1 {
//                         host.1 = false
//                     }
//                 }
//             };
//
//             ui.separator();
//
//             ScrollArea::vertical().id_source("Host Filter").min_scrolled_height(ui.available_height()).show(ui, |ui| {
//                 ui.set_width(ui.available_width());
//                 for i in 0..self.hosts.len() {
//                     let host = &mut self.hosts[i];
//                     let host_name = &host.0.clone();
//                     let checkbox = ui.checkbox(&mut host.1, format!("{} ({})", host_name, host.2));
//                     checkbox.context_menu(|ui| {
//                         if ui.button(format!("Show {} links only", host_name)).clicked() {
//                             for host in self.hosts.iter_mut() {
//                                 host.1 = &host.0 == host_name;
//                             };
//                             ui.close_menu();
//                         }
//                     });
//                 }
//             });
//         });
//     }
//
//     pub fn update_hosts(&mut self, links: &Vec<MultiUpLink>) {
//         let mut hosts: BTreeMap<String, u32> = BTreeMap::new();
//         for link in links {
//             match link {
//                 MultiUpLink::Project(project) => {
//                     if let Some(Ok(_)) = &project.status {
//                         for link in project.download_links.as_ref().unwrap() {
//                             if let Some(Ok(_)) = &link.status {
//                                 for link in link.direct_links.as_ref().unwrap() {
//                                     hosts.entry(link.host.to_string()).and_modify(|count| *count += 1).or_insert(1);
//                                 }
//                             }
//
//                         }
//                     }
//                 }
//                 MultiUpLink::Download(download) => {
//                     if let Some(Ok(_)) = &download.status {
//                         if let Some(download_links) = download.direct_links.as_ref() {
//                             for link in download_links {
//                                 hosts.entry(link.host.to_string()).and_modify(|count| *count += 1).or_insert(1);
//                             }
//                         }
//                     }
//                 }
//             }
//         }
//         self.hosts = hosts.into_iter().map(|(host, count)| (host, true, count)).collect();
//     }
//     pub fn filter_links(&self, download_link: &DownloadLink) -> Vec<String> {
//         let displayed_links: Vec<String> = vec![];
//         return match &download_link.direct_links {
//             None => displayed_links,
//             Some(links) => {
//                 links.iter()
//                     .filter(|link| {
//                         let host_check = self.hosts.iter().any(|(host_name, checked, _count)| {
//                            &link.host == host_name && *checked
//                         });
//
//                         let validity_match = match link.validity.as_str() {
//                             "valid" => self.valid,
//                             "invalid" => self.invalid,
//                             "unknown" => self.unknown,
//                             _ => self.unchecked,
//                         };
//
//                         host_check && validity_match
//                     })
//                     .map(|link| link.url.clone())
//                     .collect()
//             }
//         }
//     }
// }
