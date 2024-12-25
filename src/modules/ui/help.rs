use crate::modules::app::{DOCUMENTATION, GITHUB, HOMEPAGE};
use crossbeam_channel::{Receiver, Sender};
use eframe::egui::{Button, Context, ScrollArea, Window};
use self_update::update::Release;
use self_update::version::bump_is_greater;
use std::thread;
use crate::modules::config::{read_config, save_config};

#[derive(Default)]
pub enum UpdateStatus {
    #[default]
    Unchecked,
    Checking,
    Outdated,
    Updated,
    Error(String),
}

struct UpdateChannels {
    pub release_sender: Sender<Result<(Release, bool), String>>,
    pub release_receiver: Receiver<Result<(Release, bool), String>>,
    pub update_status_sender: Sender<Result<(), String>>,
    pub update_status_receiver: Receiver<Result<(), String>>,
}

impl Default for UpdateChannels {
    fn default() -> Self {
        let (release_sender, release_receiver) = crossbeam_channel::bounded(1);
        let (update_status_sender, update_status_receiver) = crossbeam_channel::bounded(1);
        Self {
            release_sender,
            release_receiver,
            update_status_sender,
            update_status_receiver,
        }
    }
}

pub struct HelpUI {
    pub show_help: bool,
    pub show_update: bool,
    check_for_update_on_startup: bool,
    channels: UpdateChannels,
    update_status: UpdateStatus,
    latest_version: (Release, bool),
    updating: bool,
    updating_status: String,
    pub link_to_latest_version: String,
}

impl Default for HelpUI {
    fn default() -> Self {
        Self {
            show_help: false,
            show_update: read_config().unwrap_or_default().check_for_updates_on_startup,
            check_for_update_on_startup: read_config().unwrap_or_default().check_for_updates_on_startup,
            channels: UpdateChannels::default(),
            update_status: UpdateStatus::default(),
            latest_version: (Release::default(), false),
            updating: false,
            updating_status: String::new(),
            link_to_latest_version: String::new(),
        }
    }
}

impl HelpUI {
    pub fn show_help_window(&mut self, ctx: &Context) {
        Window::new("Help").open(&mut self.show_help).show(ctx, |ui| ScrollArea::vertical().min_scrolled_height(ui.available_height()).id_salt("Help").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.hyperlink_to("Tips & Tricks and Documentation", DOCUMENTATION);
                ui.label("|");
                ui.hyperlink_to("Homepage", HOMEPAGE);
                ui.label("|");
                ui.hyperlink_to("GitHub", GITHUB);
            });

            ui.heading("Extract");
            ui.label("Extracts direct links from MultiUp or Mirrored. MultiUp requires intervention \
            for at least the first link in order to pass the Cloudflare protection. You may refresh the \
            other pages after you have passed the Captcha, but they may appear again.\n\n\
            Links are detected using RegEx, so more complicated text such as HTML can be pasted. \
            Duplicate links will be filtered out, excluding links in projects. A list of passwords \
            can be chosen for password protected links. \n\n\
            Use CTRL and SHIFT to quickly select multiple links.");

            ui.separator();

            ui.heading("Debrid");
            ui.label("Unlocks links using a Debrid service (AllDebrid and Real-Debrid).");
            
            // ui.separator();
            // 
            // ui.heading("Upload");
            // ui.label("Uploads content to MultiUp.\n\n\
            // Remote uploaded with data streaming enabled allows for better support of different sites, including Debrid services. \
            // Since this is an experimental feature, be careful when uploading large files.\n\
            // Data streaming essentially downloads and uploads chunks of data, as if the file was downloaded \
            // to disk and then uploaded to MultiUp. However, in this case, the data is not written to disk.");
        }));
    }

    pub fn show_update_window(&mut self, ctx: &Context) {
        Window::new("Updates")
            .open(&mut self.show_update)
            .show(ctx, |ui| {
                if ui.checkbox(&mut self.check_for_update_on_startup, "Check for updates on startup").clicked() {
                    let mut config = read_config().unwrap_or_default();
                    config.check_for_updates_on_startup = self.check_for_update_on_startup;
                    save_config(&config);
                };
                
                ui.horizontal(|ui| {
                    ui.heading({
                        match &self.update_status {
                            UpdateStatus::Unchecked | UpdateStatus::Checking => {
                                "Checking for updates...".to_string()
                            }
                            UpdateStatus::Outdated => "There is an update available!".to_string(),
                            UpdateStatus::Updated => "You are up-to-date!".to_string(),
                            UpdateStatus::Error(error) => format!("Update failed: {}", error),
                        }
                    });

                    if let UpdateStatus::Checking = self.update_status {
                        ui.spinner();
                    };
                });

                ui.hyperlink_to("Homepage", HOMEPAGE);

                match self.update_status {
                    UpdateStatus::Unchecked => {
                        let release_sender = self.channels.release_sender.clone();
                        thread::spawn(move || {
                            match HelpUI::check_for_updates() {
                                Ok(releases) => {
                                    let _ = release_sender.send(Ok(releases));
                                }
                                Err(error) => {
                                    let _ = release_sender.send(Err(error.to_string()));
                                }
                            };
                        });
                        self.update_status = UpdateStatus::Checking;
                    }

                    UpdateStatus::Checking => {
                        if let Ok(update) = self.channels.release_receiver.try_recv() {
                            match update {
                                Ok(update) => {
                                    self.latest_version = update;

                                    if self.latest_version.1 {
                                        self.update_status = UpdateStatus::Outdated;
                                    } else {
                                        self.update_status = UpdateStatus::Updated;
                                    }
                                }
                                Err(error) => {
                                    self.update_status = UpdateStatus::Error(error);
                                }
                            }
                        }
                    }
                    _ => {}
                };

                if let Ok(status) = self.channels.update_status_receiver.try_recv() {
                    match status {
                        Ok(_) => {
                            self.updating = false;
                            self.updating_status = "success".to_string();
                        }
                        Err(error) => {
                            self.updating = false;
                            self.updating_status = error;
                        }
                    }
                }

                if self.latest_version.1 {
                    ui.label(format!(
                        "Update available from v{} -> v{}",
                        env!("CARGO_PKG_VERSION"),
                        self.latest_version.0.version
                    ));
                    if ui
                        .add_enabled(!self.updating, Button::new("Update"))
                        .clicked()
                    {
                        self.updating = true;
                        let update_status_sender = self.channels.update_status_sender.clone();
                        let release_sender = self.channels.release_sender.clone();
                        thread::spawn(move || {
                            match HelpUI::update() {
                                Ok(app) => {
                                    let _ = update_status_sender.send(Ok(app));
                                }
                                Err(error) => {
                                    let _ = update_status_sender.send(Err(error.to_string()));
                                }
                            };

                            match HelpUI::check_for_updates() {
                                Ok(releases) => {
                                    let _ = release_sender.send(Ok(releases));
                                }
                                Err(error) => {
                                    let _ = release_sender.send(Err(error.to_string()));
                                }
                            };
                        });
                    }

                    if !self.updating_status.is_empty() {
                        if self.updating_status == "success" {
                            ui.label("Please restart the application to use the latest version!");
                        } else {
                            ui.label(format!("Error updating creator: {}", self.updating_status));
                        }
                    }

                    if let Some(body) = &self.latest_version.0.body {
                        if !body.is_empty() {
                            ui.heading("What's New");
                            ui.label(body);
                        }
                    }
                }
            });
    }

    fn check_for_updates() -> Result<(Release, bool), Box<dyn std::error::Error>> {
        let app_current_version = env!("CARGO_PKG_VERSION").to_string();
        let multiup_direct_update = self_update::backends::github::Update::configure()
            .repo_owner("Reddiepoint")
            .repo_name("MultiUp-Direct")
            .target("")
            .bin_name("MultiUp-Direct")
            .current_version(&app_current_version)
            .build()?
            .get_latest_release()?;

        let is_app_update_greater =
            bump_is_greater(&app_current_version, &multiup_direct_update.version).unwrap();

        Ok((multiup_direct_update, is_app_update_greater))
    }

    fn update() -> Result<(), Box<dyn std::error::Error>> {
        self_update::backends::github::Update::configure()
            .repo_owner("Reddiepoint")
            .repo_name("MultiUp-Direct")
            .target(match std::env::consts::OS {
                "linux" => "amd64",
                "macos" => "darwin",
                _ => "",
            })
            .bin_name("MultiUp-Direct")
            .show_download_progress(false)
            .show_output(false)
            .no_confirm(true)
            .current_version(env!("CARGO_PKG_VERSION"))
            .build()?
            .update()?;
        Ok(())
    }
}
