use std::string::ToString;
use std::thread;
use eframe::egui;
use eframe::egui::{Context};
use once_cell::sync::Lazy;
use scraper::Selector;
use crate::constants::help::HELP_MESSAGE;
use crate::functions::download::get_html;

pub struct Help {}


const VERSION_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse(r#"#pagecontent > table:nth-child(3) > tbody > tr:nth-child(3) > td:nth-child(2) > table > tbody > tr > td > div > span:nth-child(22) > span"#).unwrap());
const HOMEPAGE: String = "https://cs.rin.ru/forum/viewtopic.php?f=14&p=2822500#p2822500".to_string();
impl Help {
    pub fn show_help(ctx: &Context, open: &mut bool) {
        egui::Window::new("Help").open(open).show(ctx, |ui| {
            ui.label(HELP_MESSAGE)
        });
    }

    pub fn check_for_updates() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = reqwest::Client::new();
        thread::spawn(move || {
            rt.block_on(async {
                let website_html = get_html(&HOMEPAGE, &client).await.unwrap();
                let website_html = scraper::Html::parse_document(&website_html);
                let element = website_html.select(&VERSION_SELECTOR).next().unwrap();
                println!("{:?}", element.inner_html());
            });

        });
    }
}
