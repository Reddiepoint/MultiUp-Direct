use std::collections::HashMap;
use eframe::egui::{Context, ScrollArea, Separator, Ui};
use serde_json::to_string;
use crate::functions::hosts::get_available_hosts;
use crate::structs::download::Download;
use crate::structs::hosts::Link;


pub struct FilterLinksCriteria {
    pub valid: bool,
    pub invalid: bool,
    pub unknown: bool,
    pub hosts: Vec<(String, bool)>
}

impl Default for FilterLinksCriteria {
    fn default() -> Self {
        FilterLinksCriteria {
            valid: true,
            invalid: false,
            unknown: false,
            hosts: vec![]
        }
    }
}

impl FilterLinksCriteria {
    pub fn show(ui: &mut Ui, filter: &mut FilterLinksCriteria) {
        ui.vertical(|ui| {
            ui.set_max_width(150.0);
            ui.label("Host status: ");
            ui.checkbox(&mut filter.valid, "Valid");
            ui.checkbox(&mut filter.invalid, "Invalid");
            ui.checkbox(&mut filter.unknown, "Unknown");
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
            ScrollArea::vertical().min_scrolled_height(ui.available_height()).id_source("Host Filter").show(ui, |ui| {
                for host in filter.hosts.iter_mut() {
                    ui.checkbox(&mut host.1, host.0.clone());
                }
            });
        });
    }
}