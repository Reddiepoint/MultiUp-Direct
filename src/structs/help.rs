use crate::constants::help::{HELP_MESSAGE, VERSION};
use crate::functions::download::get_html;
use crossbeam_channel::{Receiver, Sender};
use eframe::egui;
use eframe::egui::Context;
use once_cell::sync::Lazy;
use scraper::Selector;
use std::cmp::Ordering;
use std::string::ToString;
use std::thread;

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
}

static VERSION_SELECTOR: Lazy<Selector> = Lazy::new(|| {
    Selector::parse(r#"#pagecontent > table:nth-child(3) > tbody > tr:nth-child(3) > td:nth-child(2) > table > tbody > tr > td > div > span:nth-child(22) > span"#).unwrap()
});

static CHANGELOG_SELECTOR: Lazy<Selector> = Lazy::new(|| {
    Selector::parse(r#"#pagecontent > table:nth-child(3) > tbody > tr:nth-child(3) > td:nth-child(2) > table > tbody > tr > td > div > ul:nth-child(29)"#).unwrap()
});

const HOMEPAGE: &str = "https://cs.rin.ru/forum/viewtopic.php?f=14&p=2822500#p2822500";
impl Help {
    pub fn show_help(ctx: &Context, open: &mut bool) {
        egui::Window::new("Help")
            .open(open)
            .show(ctx, |ui| ui.label(HELP_MESSAGE));
    }

    pub fn show_update(ctx: &Context, help: &mut Help) {
        egui::Window::new("Updates")
            .open(&mut help.show_update)
            .show(ctx, |ui| {
                ui.heading({
                    match help.update_status {
                        UpdateStatus::NotChecked => "Checking for updates...",
                        UpdateStatus::Checking => "Checking for updates...",
                        UpdateStatus::NotLatest => "There is an update available!",
                        UpdateStatus::SameVersion => "You are up-to-date!",
                    }
                });
                ui.hyperlink_to("Homepage", &HOMEPAGE);
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
        thread::spawn(move || {
            rt.block_on(async {
                let website_html = get_html(HOMEPAGE, &client).await.unwrap();
                let website_html = scraper::Html::parse_document(&website_html);
                let latest_version = website_html.select(&VERSION_SELECTOR).next().unwrap();
                let changelog = website_html.select(&CHANGELOG_SELECTOR).next().unwrap();

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
