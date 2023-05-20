use std::collections::BTreeMap;
use std::sync::mpsc;
use crossbeam_channel::{Sender, SendError};
use eframe::egui::Key::P;
use eframe::egui::TextBuffer;
use once_cell::sync::Lazy;
use reqwest::{Client, Error};

use crate::functions::filter::set_filter_hosts;
use crate::functions::hosts::check_validity;
use crate::structs::hosts::{DirectLink, LinkValidityResponse};

static MULTIUP_REGEX: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r"^https://multiup\.org/en/mirror/[^/]+/[^/]+$").unwrap());
/// Convert short and long links to the en/mirror page. Removes duplicates
pub fn fix_mirror_links(multiup_links: &str) -> Vec<String> {
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
    mirror_links
}

pub async fn generate_direct_links(links: Vec<String>, check_status: bool, number_of_links_tx: Sender<u8>, link_info_tx: Sender<Vec<LinkValidityResponse>>) -> (Vec<DirectLink>, Vec<(String, bool)>) {
    let (tx, rx) = crossbeam_channel::unbounded();
    let client = reqwest::Client::new();
    for (order, link) in links.iter().enumerate() {
        let tx = tx.clone();
        let temp_link = link.clone();
        let client = client.clone();
        tokio::spawn(async move {
            let generated_links = scrape_link(&temp_link, check_status, &client).await;
            tx.send((order, generated_links)).unwrap();
        });
    }
    drop(tx);

    let mut direct_links: Vec<DirectLink> = vec![];
    let mut unordered_links: Vec<(usize, (Vec<DirectLink>, Option<LinkValidityResponse>))> = vec![];
    for (order, received_links) in rx {
        let index = unordered_links.binary_search_by_key(&order, |&(o, _)| o).unwrap_or_else(|x| x);
        unordered_links.insert(index, (order, received_links));
        let responses: Vec<LinkValidityResponse> = unordered_links.iter().filter_map(|(_, (_, response))| response.clone()).collect();
        link_info_tx.send(responses);
        number_of_links_tx.send(1);
    }

    for (_, (mut links, _)) in unordered_links {
        // sort the links by name_host in place
        links.sort_by_key(|link| link.name_host.clone());
        direct_links.extend(links);
    }

    let filter_hosts = set_filter_hosts(&direct_links);

    (direct_links, filter_hosts)
}



async fn scrape_link(mirror_link: &str, check_status: bool, client: &Client) -> (Vec<DirectLink>, Option<LinkValidityResponse>) {
    let link_hosts = scrape_link_for_hosts(mirror_link, client).await;
    if link_hosts.is_empty() {
        return (vec![DirectLink::new("error".to_string(), "Invalid link".to_string(), "invalid".to_string())], None);
    }
    if !check_status {
        return (link_hosts, None);
    }
    let hosts = check_validity(mirror_link).await;
    (link_hosts.into_iter().map(|link| {
        let status = match hosts.hosts.get(&link.name_host).unwrap() {
            Some(validity) => validity,
            None => "unknown"
        };
        DirectLink::new(link.name_host, link.url, status.to_string())
    }).collect(), Some(hosts))
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