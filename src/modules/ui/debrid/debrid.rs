use crate::modules::config;
use crate::modules::config::{Config, ConfigError};
use crate::modules::debrid_services::links::DebridServiceLinks;
use crate::modules::debrid_services::services::{DebridAPIKeys, DebridService};
use crate::modules::ui::toasts::{new_error_toast, new_info_toast, new_success_toast, new_toasts};
use crossbeam_channel::Receiver;
use eframe::egui::{ComboBox, Context, ScrollArea, TextEdit, Ui};
use egui_toast::Toasts;

#[derive(Default)]
pub struct DebridUI {
    toasts: Toasts,
    // Channels
    links_receiver: Option<Receiver<Vec<DebridServiceLinks>>>,
    // Service
    debrid_service: DebridService,
    api_keys: DebridAPIKeys,
    use_remote_traffic: bool,
    // Input
    hosting_links: String,
    // Output
    debrid_links: Vec<DebridServiceLinks>,
    debrid_links_display: String,
}

impl DebridUI {
    pub fn display(ctx: &Context, ui: &mut Ui, debrid_ui: &mut DebridUI) {
        debrid_ui.toasts = new_toasts();
        debrid_ui.display_input_area(ui);
        debrid_ui.display_debrid_links_area(ui);
        debrid_ui.toasts.show(ctx);
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
                    DebridService::AllDebrid => "AllDebrid",
                    DebridService::RealDebrid => "RealDebrid",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.debrid_service,
                        DebridService::AllDebrid,
                        "AllDebrid",
                    );
                    ui.selectable_value(
                        &mut self.debrid_service,
                        DebridService::RealDebrid,
                        "RealDebrid",
                    );
                });

            let _label = ui.label("API Key:");
            match self.debrid_service {
                DebridService::AllDebrid => {
                    ui.add(
                        TextEdit::singleline(&mut self.api_keys.alldebrid)
                            .hint_text("Enter your AllDebrid API key here")
                            .password(true),
                    );
                }
                DebridService::RealDebrid => {
                    ui.add(
                        TextEdit::singleline(&mut self.api_keys.real_debrid)
                            .hint_text("Enter your Real-Debrid API key here")
                            .password(true),
                    );
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
                match self.debrid_service {
                    DebridService::AllDebrid => match self.api_keys.alldebrid.is_empty() {
                        true => config.alldebrid_api_key = None,
                        false => config.alldebrid_api_key = Some(self.api_keys.alldebrid.clone()),
                    },
                    DebridService::RealDebrid => match self.api_keys.real_debrid.is_empty() {
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
        if ui.button("Unlock links").clicked() {
            self.debrid_service.unlock_links(&self.hosting_links)
        }
    }
    pub fn display_error_log(&mut self, ctx: &Context) {}
    fn display_debrid_links_area(&mut self, ui: &mut Ui) {
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
}
