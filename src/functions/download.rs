use std::collections::BTreeMap;
use std::sync::mpsc;
use crossbeam_channel::{Sender, SendError};
use eframe::egui::Key::P;
use eframe::egui::TextBuffer;
use once_cell::sync::Lazy;
use reqwest::{Client, Error};

use crate::functions::filter::set_filter_hosts;
use crate::functions::hosts::check_validity;
use crate::structs::hosts::{DirectLink, LinkInformation, MirrorLink};

static MULTIUP_REGEX: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r"^https://multiup\.org/en/mirror/[^/]+/[^/]+$").unwrap());
/// Convert short and long links to the en/mirror page. Removes duplicates
pub fn fix_multiup_links(multiup_links: &str) -> Vec<MirrorLink> {
    let mut mirror_links = Vec::with_capacity(multiup_links.lines().count()); // Pre-allocate memory for the vector
    //let mut mirror_links = vec![];
    let mirror_prefix = "https://multiup.org/en/mirror/";
    for line in multiup_links.lines() {
        let multiup_link = line.trim().split(' ').next().unwrap();
        let multiup_link = multiup_link.replace("www.", ""); // Compatibility for older links

        let prefixes = ["https://multiup.org/download/", "https://multiup.org/en/download/", "https://multiup.org/"];
        let fixed_link = if MULTIUP_REGEX.is_match(&multiup_link) {
            multiup_link
        } else {
            let mut mirror_link = String::new();
            for prefix in prefixes {
                if let Some(suffix) = multiup_link.strip_prefix(prefix) {
                    let mut fixed_link = format!("{}{}", mirror_prefix, suffix);
                    if !MULTIUP_REGEX.is_match(&fixed_link) {
                        fixed_link.push_str("/a");
                    };
                    mirror_link = fixed_link;
                    break
                }
            };
            mirror_link
        };

        if !fixed_link.is_empty() && !mirror_links.contains(&fixed_link) {
            mirror_links.push(fixed_link)
        }
    };
    let mirror_links = mirror_links.iter().map(|link| MirrorLink::new(link.to_string())).collect();
    mirror_links
}

pub async fn generate_direct_links(mirror_links: &mut [MirrorLink], recheck_status: bool, direct_links_tx: Sender<(usize, MirrorLink)>) {
    let client = Client::new();
    let mut tasks = Vec::new();
    for (order, link) in mirror_links.iter().enumerate() {
        let direct_links_tx = direct_links_tx.clone();
        let mut mirror_link = link.clone();
        let client = client.clone();
        tasks.push(tokio::spawn(async move {
            let mirror_link = scrape_link(&mut mirror_link, recheck_status, &client).await;
            let _ = direct_links_tx.send((order, mirror_link));
        }));
    }

    for task in tasks {
        let _ = task.await;
    }
}



async fn scrape_link(mirror_link: &mut MirrorLink, check_status: bool, client: &Client) -> MirrorLink {
    let link_hosts = scrape_link_for_hosts(&mirror_link.url, client).await;
    if link_hosts.is_empty() {
        mirror_link.direct_links = Some(vec![DirectLink::new("error".to_string(), "Invalid link".to_string(), "invalid".to_string())]);
        return mirror_link.clone()
    }
    if !check_status {
        mirror_link.direct_links = Some(link_hosts);
        return mirror_link.clone();
    }
    let link_information = check_validity(&mirror_link.url).await;
    let mut direct_links: Vec<DirectLink> = link_hosts.into_iter().map(|link| {
        let status = match link_information.hosts.get(&link.name_host).unwrap() {
            Some(validity) => validity,
            None => "unknown"
        };
        DirectLink::new(link.name_host, link.url, status.to_string())
    }).collect();
    direct_links.sort_by_key(|link| link.name_host.clone());
    mirror_link.direct_links = Some(direct_links);
    mirror_link.information = Some(link_information);
    mirror_link.clone()

}

static SELECTOR: Lazy<scraper::Selector> = Lazy::new(|| scraper::Selector::parse(r#"button[type="submit"]"#).unwrap());
async fn scrape_link_for_hosts(url: &str, client: &Client) -> Vec<DirectLink> {
    // Regular links
    let mut links: Vec<DirectLink> = vec![];
    // Scrape panel
    let website_html = match get_html(url, client).await {
        Ok(html) => html,
        Err(error) => return vec![DirectLink::new("error".to_string(), error.to_string(), "invalid".to_string())]
    };

    let website_html = scraper::Html::parse_document(&website_html);
    for element in website_html.select(&SELECTOR) {
        let name_host = element.value().attr("namehost").unwrap();
        let link = element.value().attr("link").unwrap();
        let validity = element.value().attr("validity").unwrap();
        links.push(DirectLink::new(name_host.to_string(), link.to_string(), validity.to_string()))
    };

    links // Will be empty if invalid page
}

async fn get_html(url: &str, client: &Client) -> Result<String, reqwest::Error> {
    //client.get(url).await?.text().await
    client.get(url).send().await?.text().await
}