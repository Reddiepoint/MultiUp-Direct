use std::cmp::Ordering;
use std::string::ToString;
use std::sync::OnceLock;
use std::thread;

use crossbeam_channel::{Receiver, Sender};
use eframe::egui;
use eframe::egui::{Context, ScrollArea};
use scraper::{Element, Selector};

use crate::modules::download::get_html;

#[derive(Default)]
pub enum UpdateStatus {
    #[default]
    NotChecked,
    Checking,
    NotLatest,
    SameVersion,
}

#[derive(Default)]
pub struct Help {
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
pub const HELP_MESSAGE: &str = "Instructions:\n\
1. Paste your MultiUp links into the first box.\n\
You can paste as many links as you want, separated by a new line. \
As long as a line starts with a link, it will be recognised. \
This means you can paste a page with links (e.g., FitGirl). \
Duplicate links will also be filtered out.\n\n\
2. You can check the \"Re-check host status\" box if you want MultiUp to verify the availability of the hosts.\n\
This will give you more information about the files, which will appear in a section called \"Link Information\".\n\
The information about the files retrieved without this feature enabled is much more limited.\n\
Note that enabling this feature will cause the generation times to be much longer.\n\n\
3. Click on \"Generate links\" to get the direct links.\n\
You can see the progress of the generation in the status below the box.\n\n\
4. Select the links you want to use. You can do this by:\n\t\
- Holding down CTRL to select individual links.\n\t\
- Clicking and holding SHIFT to select a range of links.\n\n\
5. Right-click on a link or selection of links to see more options, such as copying or opening the links in your browser.\n\n\
6. Use the filter menu to narrow down your choices:\n\t\
- \"Unknown\": These are the links that MultiUp could not check after verification.\n\t\
- \"Unchecked\": These are the links that were not verified by MultiUp. (Links will only appear here if you do not check the \"Re-check host status\" box).\n\t\
- Hosts: You can choose which hosts you want to see links for. You can right-click on a host and select \"Select ____ links only\" to quickly filter out the rest.\n\n\
7. You can choose to hide direct links associated with a MultiUp link by deselecting it in \"Link Information\".\n\
Like the filter menu, you can right-click on a checkbox to see the context menu to select or deselect all the links, \
and you can use SHIFT to select a range of links to display.";

pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");

impl Help {
    pub fn new_channels(&mut self, tx: Sender<(String, Vec<String>)>, rx: Receiver<(String, Vec<String>)>) {
        self.update_sender = Some(tx);
        self.update_receiver = Some(rx);
    }

    pub fn show_help(ctx: &Context, open: &mut bool) {
        egui::Window::new("Help")
            .open(open)
            .show(ctx, |ui| ScrollArea::vertical().min_scrolled_height(ui.available_height()).id_source("Help").show(ui, |ui| {
                ui.label(HELP_MESSAGE);
            }));
    }

    pub fn show_update(ctx: &Context, help: &mut Help) {
        egui::Window::new("Updates")
            .open(&mut help.show_update)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading({
                        match help.update_status {
                            UpdateStatus::NotChecked => "Checking for updates...",
                            UpdateStatus::Checking => "Checking for updates...",
                            UpdateStatus::NotLatest => "There is an update available!",
                            UpdateStatus::SameVersion => "You are up-to-date!",
                        }
                    });

                    if let UpdateStatus::Checking = help.update_status {
                        ui.spinner();
                    };
                });


                ui.hyperlink_to("Homepage", HOMEPAGE);
                let mut changelog_text = String::new();
                for change in help.latest_changelog.iter() {
                    changelog_text.push_str(&format!("- {}\n", change));
                };

                if !changelog_text.is_empty() {
                    ui.separator();
                    ui.heading(format!("What's new in v{}", help.latest_version));
                    ui.label(changelog_text);
                }

                match help.update_status {
                    UpdateStatus::NotChecked => {
                        Help::is_updated(help.update_sender.clone().unwrap());
                        help.update_status = UpdateStatus::Checking;
                    }
                    UpdateStatus::NotLatest => {}
                    UpdateStatus::SameVersion => {}
                    UpdateStatus::Checking => {
                        if let Ok((latest_version, changelog)) = help.update_receiver.clone().unwrap().try_recv() {
                            let app_version: Vec<u32> = latest_version
                                .split('.')
                                .map(|s| s.parse().unwrap())
                                .collect();
                            let homepage_version: Vec<u32> =
                                VERSION.split('.').map(|s| s.parse().unwrap()).collect();
                            match app_version.cmp(&homepage_version) {
                                Ordering::Less => help.update_status = UpdateStatus::NotLatest,
                                Ordering::Equal => help.update_status = UpdateStatus::SameVersion,
                                Ordering::Greater => help.update_status = UpdateStatus::NotLatest,
                            };
                            help.latest_changelog = changelog;
                            help.latest_version = latest_version;
                        }
                    }
                };
            });
    }

    pub fn is_updated(tx: Sender<(String, Vec<String>)>) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/39.0.2171.71 Safari/537.36 Edge/12.0")
            .build()
            .unwrap();
        let version_selector = VERSION_SELECTOR.get_or_init(|| Selector::parse(r#"span[style="color: #19EC1C"] span[style="font-weight: bold"]"#).unwrap());
        let changelog_selector = CHANGELOG_SELECTOR.get_or_init(|| Selector::parse(r#"span[style="font-weight: bold"] > span[style="color: #E93C1C"]"#).unwrap());
        thread::spawn(move || {
            rt.block_on(async {
                let website_html = get_html(HOMEPAGE, &client).await.unwrap();
                let website_html = scraper::Html::parse_document(&website_html);
                let latest_version = website_html.select(version_selector).next().unwrap();
                let changelog = website_html.select(changelog_selector)
                    .find(|element| element.text().collect::<Vec<_>>().join("") == "Changelog").unwrap()
                    .parent_element().unwrap()
                    .next_sibling_element().unwrap()
                    .next_sibling_element().unwrap();
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
