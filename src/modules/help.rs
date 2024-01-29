use std::cmp::Ordering;
use std::sync::OnceLock;
use std::thread;
use crossbeam_channel::{Receiver, Sender};
use eframe::egui::{Context, ScrollArea, Window};
use reqwest::Client;
use scraper::{Element, Selector};
use crate::modules::general::get_page_html;

#[derive(Default)]
pub enum UpdateStatus {
    #[default]
    Unchecked,
    Checking,
    Outdated,
    Updated
}

#[derive(Default)]
pub struct HelpUI {
    pub show_help: bool,
    pub show_update: bool,
    pub update_sender: Option<Sender<(String, Vec<String>)>>,
    pub update_receiver: Option<Receiver<(String, Vec<String>)>>,
    pub update_status: UpdateStatus,
    pub latest_changelog: Vec<String>,
    pub latest_version: String,
    pub link_to_latest_version: String,
}

static VERSION_SELECTOR: OnceLock<Selector> = OnceLock::new();
static CHANGELOG_SELECTOR: OnceLock<Selector> = OnceLock::new();

const HOMEPAGE: &str = "https://cs.rin.ru/forum/viewtopic.php?f=14&p=2822500#p2822500";
const DOCUMENTATION: &str = "https://reddiepoint.github.io/MultiUp-Direct-Documentation/";

pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");

impl HelpUI {
    pub fn update_channels(&mut self, tx: Sender<(String, Vec<String>)>, rx: Receiver<(String, Vec<String>)>) {
        self.update_sender = Some(tx);
        self.update_receiver = Some(rx);
    }

    pub fn show_help(ctx: &Context, help_ui: &mut HelpUI) {
        Window::new("Help").open(&mut help_ui.show_help).show(ctx, |ui| ScrollArea::vertical().min_scrolled_height(ui.available_height()).id_source("Help").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.hyperlink_to("Documentation", DOCUMENTATION);
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
            ui.label("Unlocks links using a Debrid service.\n\
            Currently supports AllDebrid and RealDebrid.\n\
            To read the keys from a file, create \"api_key.json\" in the same directory as this app with the following structure:");
            let mut json_example = "\
            {\n\
                \"all_debrid\": \"YOUR_ALLDEBRID_API_KEY\",\n\
                \"real_debrid\": \"YOUR_REALDEBRID_API_KEY\"\n\
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

    pub fn show_update(ctx: &Context, help_ui: &mut HelpUI) {
        Window::new("Updates").open(&mut help_ui.show_update).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading({
                    match help_ui.update_status {
                        UpdateStatus::Unchecked => "Checking for updates...",
                        UpdateStatus::Checking => "Checking for updates...",
                        UpdateStatus::Outdated => "There is an update available!",
                        UpdateStatus::Updated => "You are up-to-date!",
                    }
                });

                if let UpdateStatus::Checking = help_ui.update_status {
                    ui.spinner();
                };
            });


            ui.hyperlink_to("Homepage", HOMEPAGE);
            let mut changelog_text = String::new();
            for change in help_ui.latest_changelog.iter() {
                changelog_text.push_str(&format!("- {}\n", change));
            };

            if !changelog_text.is_empty() {
                ui.separator();
                ui.heading(format!("What's new in v{}", help_ui.latest_version));
                ui.label(changelog_text);
            }

            match help_ui.update_status {
                UpdateStatus::Unchecked => {
                    HelpUI::is_updated(help_ui.update_sender.clone().unwrap());
                    help_ui.update_status = UpdateStatus::Checking;
                }
                UpdateStatus::Outdated => {}
                UpdateStatus::Updated => {}
                UpdateStatus::Checking => {
                    if let Ok((latest_version, changelog)) = help_ui.update_receiver.clone().unwrap().try_recv() {
                        let version = VERSION.split('-').next().unwrap().to_string();
                        let app_version: Vec<u32> = version.split('.').map(|s| s.parse().unwrap()).collect();
                        let homepage_version: Vec<u32> = latest_version.split('.').map(|s| s.parse().unwrap()).collect();
                        match app_version.cmp(&homepage_version) {
                            Ordering::Less => help_ui.update_status = UpdateStatus::Outdated,
                            Ordering::Equal => help_ui.update_status = UpdateStatus::Updated,
                            Ordering::Greater => help_ui.update_status = UpdateStatus::Updated,
                        };
                        help_ui.latest_changelog = changelog;
                        help_ui.latest_version = latest_version;
                    }
                }
            };
        });
    }

    pub fn is_updated(tx: Sender<(String, Vec<String>)>) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        // let client = reqwest::Client::builder().user_agent("Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/39.0.2171.71 Safari/537.36 Edge/12.0").build().unwrap();
        let version_selector = VERSION_SELECTOR.get_or_init(|| Selector::parse(r#"span[style="color: #19EC1C"] span[style="font-weight: bold"]"#).unwrap());
        let changelog_selector = CHANGELOG_SELECTOR.get_or_init(|| Selector::parse(r#"span[style="font-weight: bold"] > span[style="color: #E93C1C"]"#).unwrap());
        thread::spawn(move || {
            rt.block_on(async {
                let client = Client::new();
                let html = match get_page_html(HOMEPAGE, &client, None, 0).await {
                    Ok(html) => html,
                    Err(error) => {
                        let _ = tx.send(("Unknown".to_string(), vec![format!("Failed to get changelog: {:?}", error)]));
                        return;
                    }
                };
                let website_html = scraper::Html::parse_document(&html);
                let latest_version = website_html.select(version_selector).next().unwrap();
                let changelog = website_html.select(changelog_selector).find(|element| element.text().collect::<Vec<_>>().join("") == "Changelog").unwrap().parent_element().unwrap().next_sibling_element().unwrap().next_sibling_element().unwrap();
                let mut changelog_children = vec![];
                let mut changelog_points = vec![];
                for child in changelog.children() {
                    changelog_children.push(child);
                }

                while let Some(node) = changelog_children.pop() {
                    if let Some(text) = node.value().as_text() {
                        changelog_points.push(text.trim().to_string());
                    }
                    for child in node.children() {
                        changelog_children.push(child);
                    }
                };
                let mut changelog: Vec<String> = changelog_points.iter_mut().filter(|text| !text.is_empty()).map(|text| text.to_string()).collect();
                changelog.reverse();
                let _ = tx.send((latest_version.inner_html()[1..].to_string(), changelog));
            });
        });
    }
}