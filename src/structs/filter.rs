use eframe::egui::{ScrollArea, Ui};

#[derive(Clone)]
pub struct FilterMenu {
    pub valid: bool,
    pub invalid: bool,
    pub unknown: bool,
    pub unchecked: bool,
    pub hosts: Vec<(String, bool)>
}

impl Default for FilterMenu {
    fn default() -> Self {
        FilterMenu {
            valid: true,
            invalid: false,
            unknown: false,
            unchecked: false,
            hosts: vec![]
        }
    }
}

impl FilterMenu {
    pub fn show(ui: &mut Ui, filter: &mut FilterMenu) {
        ui.vertical(|ui| {
            ui.set_max_width(150.0);
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
                for i in 0..filter.hosts.len() {
                    let host = &mut filter.hosts[i];
                    let host_name = &host.0.clone();
                    let checkbox = ui.checkbox(&mut host.1, host_name);
                    checkbox.context_menu(|ui| {
                        if ui
                            .button(format!("Select {} links only", host_name))
                            .clicked()
                        {
                            for host in filter.hosts.iter_mut() {
                                if &host.0 == host_name {
                                    host.1 = true;
                                } else {
                                    host.1 = false;
                                };
                            };
                            ui.close_menu();
                        }
                    });
                }
            });
        });
    }
}