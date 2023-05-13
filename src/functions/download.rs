use std::collections::BTreeMap;
use std::sync::mpsc;
use crossbeam_channel::{Sender, SendError};
use eframe::egui::Key::P;
use once_cell::sync::Lazy;
use crate::functions::filter::set_filter_hosts;
use crate::functions::hosts::check_validity;
use crate::structs::hosts::Link;


pub async fn generate_direct_links(links: Vec<String>, check_status: bool, number_of_links_tx: Sender<u8>) -> (Vec<Link>, Vec<(String, bool)>) {
    let (tx, rx) = crossbeam_channel::bounded(100);
    for link in links {
        let tx = tx.clone();
        let temp_link = link.clone();
        tokio::spawn(async move {
            let generated_links = scrape_link(&temp_link, check_status).await;
            tx.send(generated_links).unwrap();
        });
    }
    drop(tx);
    let mut direct_links: Vec<Link> = vec![];
    for received_links in rx {
        direct_links.extend(received_links);
        number_of_links_tx.send(1);
    }

    let filter_hosts = set_filter_hosts(&direct_links);

    (direct_links, filter_hosts)
}

static RE: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r"^https://multiup\.org/en/mirror/[^/]+/[^/]+$").unwrap());
/// Convert short and long links to the en/mirror page. Removes duplicates
pub fn fix_mirror_links(links: &str) -> Vec<String> {
    let mut mirror_links = Vec::with_capacity(links.lines().count()); // Pre-allocate memory for the vector
    //let mut mirror_links = vec![];
    let prefix = "https://multiup.org/en/mirror/"; // Store the common prefix as a constant
    for link in links.lines() {
        let link = link.trim().split(' ').next().unwrap();
        let link = link.replace("www.", "");

        // Use starts_with and strip_prefix instead of replace
        let fixed_link = if link.starts_with(prefix) {
            link // No need to modify the link
        } else if let Some(suffix) = link.strip_prefix("https://multiup.org/download/"){
            let mut fixed_link = format!("{}{}", prefix, suffix); // Use format instead of replace
            if !RE.is_match(&fixed_link) {
                fixed_link.push_str("/a"); // Same as before
            };
            fixed_link
        } else if let Some(suffix) = link.strip_prefix("https://multiup.org/en/download/"){
            let mut fixed_link = format!("{}{}", prefix, suffix); // Use format instead of replace
            if !RE.is_match(&fixed_link) {
                fixed_link.push_str("/a"); // Same as before
            };
            fixed_link
        } else if let Some(suffix) = link.strip_prefix("https://multiup.org/"){
            let mut fixed_link = format!("{}{}", prefix, suffix); // Use format instead of replace
            if !RE.is_match(&fixed_link) {
                fixed_link.push_str("/a"); // Same as before
            };
            fixed_link
        } else {
            String::new() // Same as before
        };

        if !fixed_link.is_empty() && !mirror_links.contains(&fixed_link) {
            mirror_links.push(fixed_link)
        }
    };
    mirror_links
}

async fn scrape_link(mirror_link: &str, check_status: bool) -> Vec<Link> {
    let link_hosts = scrape_link_for_hosts(mirror_link).await;
    if link_hosts.is_empty() {
        return vec![Link::new("error".to_string(), "Invalid link".to_string(), "invalid".to_string())];
    }
    if !check_status {
        return link_hosts;
    }
    let hosts = check_validity(mirror_link).await;
    link_hosts.into_iter().map(|link| {
        let status = match hosts.get(&link.name_host).unwrap() {
            Some(validity) => validity,
            None => "unknown"
        };
        Link::new(link.name_host, link.url, status.to_string())
    }).collect()
}

async fn scrape_link_for_hosts(url: &str) -> Vec<Link> {
    // Regular links
    let mut links: Vec<Link> = vec![];
    // Scrape panel
    let website_html = reqwest::get(url).await.unwrap().text().await.unwrap();
    let website_html = scraper::Html::parse_document(&website_html);
    let button_selector = scraper::Selector::parse(r#"button[type="submit"]"#).unwrap();
    for element in website_html.select(&button_selector) {
        let name_host = element.value().attr("namehost").unwrap();
        let link = element.value().attr("link").unwrap();
        let validity = element.value().attr("validity").unwrap();
        links.push(Link::new(name_host.to_string(), link.to_string(), validity.to_string()))
    };

    links // Will be empty if invalid page
}