use std::thread;
use crossbeam_channel::{Receiver, Sender};
use eframe::egui::{Button, Context, ScrollArea, Window};
use self_update::update::Release;
use self_update::version::bump_is_greater;

#[derive(Default)]
pub enum UpdateStatus {
    #[default]
    Unchecked,
    Checking,
    Outdated,
    Updated,
    Error(String),
}

struct HelpChannels {
    pub release_sender: Sender<Result<(Release, bool), String>>,
    pub release_receiver: Receiver<Result<(Release, bool), String>>,
    pub update_status_sender: Sender<Result<(), String>>,
    pub update_status_receiver: Receiver<Result<(), String>>,
}

impl Default for HelpChannels {
    fn default() -> Self {
        let (release_sender, release_receiver) = crossbeam_channel::bounded(1);
        let (update_status_sender, update_status_receiver) = crossbeam_channel::bounded(1);
        Self {
            release_sender,
            release_receiver,
            update_status_sender,
            update_status_receiver
        }
    }
}

pub struct HelpUI {
    pub show_help: bool,
    pub show_update: bool,
    channels: HelpChannels,
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
            show_update: true,
            channels: HelpChannels::default(),
            update_status: UpdateStatus::default(),
            latest_version: (Release::default(), false),
            updating: false,
            updating_status: String::new(),
            link_to_latest_version: String::new(),
        }
    }
}


const HOMEPAGE: &str = "https://cs.rin.ru/forum/viewtopic.php?f=14&p=2822500#p2822500";
const DOCUMENTATION: &str = "https://reddiepoint.github.io/RedAlt-SteamUp-Documentation/using-the-creator.html";

impl HelpUI {
    pub fn show_help_window(&mut self, ctx: &Context) {
        Window::new("Help").open(&mut self.show_help).show(ctx, |ui| ScrollArea::vertical().min_scrolled_height(ui.available_height()).id_source("Help").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.hyperlink_to("Tips & Tricks and Extra Information", DOCUMENTATION);
                ui.label("|");
                ui.hyperlink_to("Homepage", HOMEPAGE);
            });

            ui.heading("Extract");
            ui.label("Extracts direct links from MultiUp links.\n\n\
            Link detection is quite robust, meaning you can paste in any page with links as well as HTML containing links. \
            Duplicate links will be filtered out, excluding links in projects.\n\n\
            If you want the validity of the hosts to be checked by MultiUp, enable \"Recheck link validity,\" \
            otherwise, the original values from the site will be used. However, generation times may take much longer if this is enabled.\n\n\
            You can select direct links by using combinations of CTRL and SHIFT and clicking and search for file names.");

            ui.separator();

            ui.heading("Debrid");
            ui.label("Unlocks links using a Debrid service.\n\n\
            Currently supports AllDebrid and RealDebrid.\n\
            To read the keys from a file, create \"api_key.json\" in the same directory as this app with the following structure:");
            let mut json_example = "\
            {\n\
                \t\"all_debrid\": \"YOUR_ALLDEBRID_API_KEY\",\n\
                \t\"real_debrid\": \"YOUR_REALDEBRID_API_KEY\"\n\
            }";
            ui.code_editor(&mut json_example);
            ui.label("You can choose to omit any field here (i.e. only have all_debrid or real_debrid) \
            if you do not have an API key for the service.");

            ui.separator();

            ui.heading("Upload");
            ui.label("Uploads content to MultiUp.\n\n\
            Remote uploaded with data streaming enabled allows for better support of different sites, including Debrid services.\
            Since this is an experimental feature, be careful when uploading large files.\n\
            Data streaming essentially downloads and uploads chunks of data, as if the file was downloaded \
            to disk and then uploaded to MultiUp. However, in this case, the data is not written to disk.");
        }));
    }

    pub fn show_update_window(&mut self, ctx: &Context) {
        Window::new("Updates").open(&mut self.show_update).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading({
                    match &self.update_status {
                        UpdateStatus::Unchecked | UpdateStatus::Checking => "Checking for updates...".to_string(),
                        UpdateStatus::Outdated => "There is an update available!".to_string(),
                        UpdateStatus::Updated => "You are up-to-date!".to_string(),
                        UpdateStatus::Error(error) => format!("Update failed: {}", error)
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
                            },
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
                            },
                            Err(error) => {
                                self.update_status = UpdateStatus::Error(error);
                            }
                        }
                    }
                },
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
                ui.label(format!("Update available from v{} -> v{}", env!("CARGO_PKG_VERSION"), self.latest_version.0.version));
                if ui.add_enabled(!self.updating, Button::new("Update")).clicked() {
                    self.updating = true;
                    let update_status_sender = self.channels.update_status_sender.clone();
                    let release_sender = self.channels.release_sender.clone();
                    thread::spawn(move || {
                        match HelpUI::update() {
                            Ok(app) => {
                                let _ = update_status_sender.send(Ok(app));
                            },
                            Err(error) => {
                                let _ = update_status_sender.send(Err(error.to_string()));
                            }
                        };

                        match HelpUI::check_for_updates() {
                            Ok(releases) => {
                                let _ = release_sender.send(Ok(releases));
                            },
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

        let is_app_update_greater = bump_is_greater(&app_current_version, &multiup_direct_update.version).unwrap();

        Ok((multiup_direct_update, is_app_update_greater))
    }

    fn update() -> Result<(), Box<dyn std::error::Error>> {
        self_update::backends::github::Update::configure()
            .repo_owner("Reddiepoint")
            .repo_name("MultiUp-Direct")
            .target(match std::env::consts::OS {
                "linux" => "amd64",
                "macos" => "darwin",
                _ => ""
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
