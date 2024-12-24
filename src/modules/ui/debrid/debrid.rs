use crate::modules::config;
use crate::modules::config::{Config, ConfigError};
use crate::modules::debrid_services::links::DebridServiceLink;
use crate::modules::debrid_services::services::{DebridAPIKeys, DebridService};
use crate::modules::ui::toasts::{new_error_toast, new_info_toast, new_success_toast, new_toasts};
use crossbeam_channel::Receiver;
use eframe::egui::{Button, ComboBox, Context, ScrollArea, TextEdit, TopBottomPanel, Ui, Window};
use egui_extras::{Column, TableBuilder};
use egui_toast::Toasts;

#[derive(Default)]
pub struct DebridUI {
    toasts: Toasts,
    // Channels
    links_receiver: Option<Receiver<Vec<DebridServiceLink>>>,
    // Service
    debrid_service: DebridService,
    api_keys: DebridAPIKeys,
    use_remote_traffic: bool,
    // Input
    hosting_links: String,
    unlocking: bool,
    // Output
    debrid_links: Vec<DebridServiceLink>,
    debrid_links_display: String,
    // Error log
    error_log_open: bool,
}

impl DebridUI {
    pub fn display(ctx: &Context, ui: &mut Ui, debrid_ui: &mut DebridUI) {
        debrid_ui.toasts = new_toasts();
        debrid_ui.receive_channels();
        debrid_ui.display_input_area(ui);
        ui.separator();
        debrid_ui.display_footer(ctx);
        debrid_ui.display_output_area(ui);
        debrid_ui.display_error_log(ctx);
        debrid_ui.toasts.show(ctx);
    }

    fn receive_channels(&mut self) {
        if let Some(links_receiver) = &self.links_receiver {
            if let Ok(links) = links_receiver.try_recv() {
                self.unlocking = false;
                self.debrid_links = links;
                self.debrid_links_display = self
                    .debrid_links
                    .iter()
                    .filter(|link| link.status.is_ok())
                    .map(|link| link.link.clone())
                    .collect::<Vec<String>>()
                    .join("\n");
            }
        }
    }

    fn display_input_area(&mut self, ui: &mut Ui) {
        ui.heading("Hosting Service Input Links");

        ScrollArea::both()
            .id_salt("Hosting Service Input Links Box")
            .max_height(ui.available_height() / 4.0)
            .show(ui, |ui| {
                ui.add(
                    TextEdit::multiline(&mut self.hosting_links)
                        .hint_text("Enter your links here")
                        .desired_width(ui.available_width()),
                );
            });

        ui.horizontal(|ui| {
            let _label = ui.label("Debrid Service:");
            let _debrid_selector = ComboBox::from_id_salt("Debrid Service Selector")
                .selected_text(match self.debrid_service {
                    DebridService::AllDebrid(_) => "AllDebrid",
                    DebridService::RealDebrid(_) => "Real-Debrid",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.debrid_service,
                        DebridService::AllDebrid(self.api_keys.alldebrid.clone()),
                        "AllDebrid",
                    );
                    ui.selectable_value(
                        &mut self.debrid_service,
                        DebridService::RealDebrid(self.api_keys.real_debrid.clone()),
                        "Real-Debrid",
                    );
                });

            let _label = ui.label("API Key:");
            match self.debrid_service {
                DebridService::AllDebrid(_) => {
                    ui.add(
                        TextEdit::singleline(&mut self.api_keys.alldebrid)
                            .hint_text("Enter your AllDebrid API key here")
                            .password(true),
                    );
                    self.debrid_service = DebridService::AllDebrid(self.api_keys.alldebrid.clone());
                }
                DebridService::RealDebrid(_) => {
                    ui.add(
                        TextEdit::singleline(&mut self.api_keys.real_debrid)
                            .hint_text("Enter your Real-Debrid API key here")
                            .password(true),
                    );
                    self.debrid_service =
                        DebridService::RealDebrid(self.api_keys.real_debrid.clone());
                }
            }

            if ui.button("Save API Key").clicked() {
                let mut config = match config::read_config() {
                    Ok(config) => config,
                    Err(error) => match error {
                        ConfigError::ConfigNotFound => {
                            new_info_toast(
                                &mut self.toasts,
                                "Config file not found. A new one will be created.",
                                None,
                                None,
                            );
                            Config::default()
                        }
                        ConfigError::Other(error) => {
                            new_error_toast(&mut self.toasts, &error, None, None);
                            Config::default()
                        }
                        _ => Config::default(),
                    },
                };
                match &self.debrid_service {
                    DebridService::AllDebrid(api_key) => match api_key.is_empty() {
                        true => config.alldebrid_api_key = None,
                        false => config.alldebrid_api_key = Some(self.api_keys.alldebrid.clone()),
                    },
                    DebridService::RealDebrid(api_key) => match api_key.is_empty() {
                        true => config.real_debrid_api_key = None,
                        false => {
                            config.real_debrid_api_key = Some(self.api_keys.real_debrid.clone())
                        }
                    },
                };

                match config::save_config(&config) {
                    Ok(_) => {
                        new_success_toast(&mut self.toasts, "API key saved.", None, None);
                    }
                    Err(error) => {
                        let error_message = match error {
                            ConfigError::InvalidJson => {
                                "Error saving config: Invalid JSON".to_string()
                            }
                            ConfigError::Other(error) => {
                                "Error saving config: ".to_string() + &error
                            }
                            _ => "Error saving config.".to_string(),
                        };

                        new_error_toast(&mut self.toasts, &error_message, None, None);
                    }
                }
            }
        });

        if self.debrid_service.supports_remote_traffic() {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.use_remote_traffic, "Use remote traffic");
                ui.label("(?)").on_hover_ui(|ui| {
                    ui.label(
                        "Generates links that do not have remote dedicated servers \
                    and account sharing protections. \
                    \nImportantly, this uses your \"remote traffic\" quota, \
                    which may incur extra costs.",
                    );
                });
            });
        }

        ui.horizontal(|ui| {
            if ui
                .add_enabled(!self.unlocking, Button::new("Unlock"))
                .clicked()
            {
                self.unlocking = true;
                let (links_sender, links_receiver) = crossbeam_channel::unbounded();
                self.links_receiver = Some(links_receiver);
                self.debrid_service.unlock_links(
                    &self.hosting_links,
                    self.use_remote_traffic,
                    links_sender,
                );
            }

            if self.unlocking {
                ui.spinner();
                ui.label("Unlocking links...");
            }

            if ui.button("See errors").clicked() {
                self.error_log_open = true;
            }
        });
    }
    pub fn display_error_log(&mut self, ctx: &Context) {
        Window::new("Debrid Error Log")
            .open(&mut self.error_log_open)
            .default_width(1000.0)
            .show(ctx, |ui| {
                let error_log: Vec<(String, String)> = self
                    .debrid_links
                    .iter()
                    .filter(|link| link.status.is_err())
                    .map(|link| {
                        (
                            link.original_link.clone(),
                            link.status.clone().unwrap_err().get_error_message(),
                        )
                    })
                    .collect();
                ScrollArea::vertical()
                    .id_salt("Debrid Error Log Box")
                    .max_width(ui.available_width() * 0.8)
                    .show(ui, |ui| {
                        let heights = error_log
                            .iter()
                            .map(|(_link, _error)| 20.0 + 18.0)
                            .collect::<Vec<f32>>();
                        ui.push_id("Debrid Error Log Table", |ui| {
                            let width = ui.available_width() / 2.0;
                            TableBuilder::new(ui)
                                .column(Column::exact(width))
                                .column(Column::exact(width))
                                .body(|body| {
                                    body.heterogeneous_rows(heights.into_iter(), |mut row| {
                                        let row_index = row.index();
                                        row.col(|ui| {
                                            ui.label(&error_log[row_index].0);
                                        });
                                        row.col(|ui| {
                                            ui.label(&error_log[row_index].1);
                                        });
                                    })
                                });
                        });
                    });
            });
    }
    fn display_output_area(&mut self, ui: &mut Ui) {
        ui.heading("Debrid Links");

        // Copy buttons
        ui.horizontal(|ui| {
            // TODO: Normal copy, copy with quotes and new line, copy with quotes and spaces
        });

        // Create uneditable copy
        let mut debrid_links = self.debrid_links_display.clone();
        ScrollArea::both()
            .id_salt("Debrid Links Box")
            .max_height(ui.available_height())
            .show(ui, |ui| {
                ui.add(TextEdit::multiline(&mut debrid_links).desired_width(ui.available_width()));
            });
    }

    fn display_footer(&mut self, ctx: &Context) {
        TopBottomPanel::bottom("Information Footer").show(ctx, |ui| {
            let total_links = self.debrid_links.len();
            let successful_links = self
                .debrid_links
                .iter()
                .map(|link| if link.status.is_ok() { 1 } else { 0 })
                .sum::<usize>();
            ui.label(format!(
                "Unlocked links: {}/{}",
                successful_links, total_links
            ));
        });
    }
}
