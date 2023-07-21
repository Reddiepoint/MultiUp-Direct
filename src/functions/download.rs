use std::sync::OnceLock;

use crossbeam_channel::{Sender, TryRecvError};
use reqwest::Client;
use scraper::Selector;
use tokio::runtime::Runtime;

use crate::functions::hosts::check_validity;
use crate::structs::download::ParsedTitle;
use crate::structs::hosts::{DirectLink, LinkInformation, MirrorLink};

static MULTIUP_REGEX: OnceLock<regex::Regex> = OnceLock::new();
static MIRROR_REGEX: OnceLock<regex::Regex> = OnceLock::new();
static PROJECT_REGEX: OnceLock<regex::Regex> = OnceLock::new();

/// Convert short and long links to the en/mirror page. Removes duplicates
pub fn fix_multiup_links(multiup_links: String) -> Vec<MirrorLink> {
    let mirror_prefix = "https://multiup.org/en/mirror/";
    let multiup_regex = MULTIUP_REGEX.get_or_init(|| regex::Regex::new(r#"^https?://(www\.)?multiup\.org/(en/)?(download/)?"#).unwrap());
    let mirror_regex = MIRROR_REGEX.get_or_init(|| regex::Regex::new(r#"^https?://multiup\.org/en/mirror/[^/]+/[^/]+$"#).unwrap());
    let project_regex = PROJECT_REGEX.get_or_init(|| regex::Regex::new(r#"^https:\/\/(www\.)?multiup\.org\/(en\/)?project\/.*$"#).unwrap());

    let mut mirror_links: Vec<String> = Vec::with_capacity(multiup_links.lines().count()); // Pre-allocate memory for the vector
    let (multiup_links_tx, multiup_links_rx) = crossbeam_channel::unbounded();
    for line in multiup_links.lines() {
        let multiup_link = line.trim().split(' ').next().unwrap().to_string();
        if mirror_regex.is_match(&multiup_link) {
            if !mirror_links.contains(&multiup_link) {
                mirror_links.push(multiup_link.to_string());
            }
        } else if project_regex.is_match(&multiup_link) {
            let rt = Runtime::new().unwrap();
            let multiup_links_tx = multiup_links_tx.clone();
            std::thread::spawn(move || {
                rt.block_on(async {
                    let multiup_links = match get_project_links(&multiup_link).await {
                        Some(project_links) => fix_multiup_links(project_links.clone()),
                        None => vec![MirrorLink::new(multiup_link.to_string())]
                    };
                    let _ = multiup_links_tx.send(multiup_links);
                });
            });
        } else if multiup_regex.is_match(&multiup_link) {
            let suffix = multiup_regex.replace(&multiup_link, "");
            let mut fixed_link = format!("{}{}", mirror_prefix, suffix);
            if mirror_regex.is_match(&fixed_link) {
                if !mirror_links.contains(&fixed_link) {
                    mirror_links.push(fixed_link);
                };
            } else {
                fixed_link.push_str("/a");
                if mirror_regex.is_match(&fixed_link) && !mirror_links.contains(&fixed_link) {
                    mirror_links.push(fixed_link);
                };
            }
        }
    };

    drop(multiup_links_tx);
    let mut mirror_links: Vec<MirrorLink> = mirror_links.iter().map(|link| MirrorLink::new(link.to_string())).collect();

    loop {
        match multiup_links_rx.try_recv() {
            Ok(mut links) => {
                mirror_links.append(&mut links)
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                // Channel is disconnected
                break;
            }
        }
    }
    mirror_links
}

static PROJECT_LINKS_SELECTOR: OnceLock<Selector> = OnceLock::new();

pub async fn get_project_links(url: &str) -> Option<String> {
    let client = Client::new();
    let html = match client.get(url).send().await.unwrap().text().await {
        Ok(html) => html,
        Err(_) => return None
    };
    let project_links_selector = PROJECT_LINKS_SELECTOR.get_or_init(|| Selector::parse(r#"#textarea-links-long"#).unwrap());
    let html = scraper::Html::parse_document(&html);
    let links = match html.select(project_links_selector).next() {
        Some(links) => links,
        None => return None
    };
    Some(links.inner_html().to_string())
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
    let mut link_hosts = scrape_link_for_hosts(&mirror_link.url, client).await;
    if link_hosts.1[0].name_host == "error" {
        let url = mirror_link.url.clone();
        mirror_link.direct_links = Some(vec![DirectLink::new("error".to_string(), "Invalid link".to_string(), "invalid".to_string())]);
        mirror_link.information = Some(LinkInformation {
            error: "invalid".to_string(),
            file_name: "Invalid link".to_string(),
            size: 0.to_string(),
            date_upload: "".to_string(),
            time_upload: 0,
            date_last_download: "N/A".to_string(),
            number_downloads: 0,
            description: Some(url),
            hosts: Default::default(),
        });
        return mirror_link.clone();
    }
    if !check_status {
        link_hosts.1.sort_by_key(|link| link.name_host.clone());
        mirror_link.direct_links = Some(link_hosts.1);
        let mut parsed_title = link_hosts.0;
        match parsed_title.unit.to_lowercase().as_str() {
            "kb" => parsed_title.size *= 1024.0,
            "mb" => parsed_title.size *= 1048576.0,
            "gb" => parsed_title.size *= 1073741824.0,
            _ => {}
        };
        parsed_title.size = parsed_title.size.floor();
        mirror_link.information = Some(LinkInformation {
            error: "success".to_string(),
            file_name: parsed_title.file_name,
            size: parsed_title.size.to_string(),
            date_upload: "".to_string(),
            time_upload: 0,
            date_last_download: "N/A".to_string(),
            number_downloads: 0,
            description: None,
            hosts: Default::default(),
        });
        return mirror_link.clone();
    }
    let link_information = check_validity(&mirror_link.url).await;
    let mut direct_links: Vec<DirectLink> = link_hosts.1.into_iter().map(|link| {
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

static SELECTOR: OnceLock<Selector> = OnceLock::new();
static FILE_NAME_SELECTOR: OnceLock<Selector> = OnceLock::new();

async fn scrape_link_for_hosts(url: &str, client: &Client) -> (ParsedTitle, Vec<DirectLink>) {
    // Regular links
    let mut links: Vec<DirectLink> = vec![];
    // Scrape panel
    let website_html = match get_html(url, client).await {
        Ok(html) => html,
        Err(error) => return (ParsedTitle::new(String::new(), 0.0, String::new()), vec![DirectLink::new("error".to_string(), error.to_string(), "invalid".to_string())])
    };
    let selector = SELECTOR.get_or_init(|| Selector::parse(r#"button[type="submit"]"#).unwrap());
    let file_name_selector = FILE_NAME_SELECTOR.get_or_init(|| Selector::parse(r#"body > section > div > section > header > h2 > a"#).unwrap());
    let website_html = scraper::Html::parse_document(&website_html);
    for element in website_html.select(selector) {
        let name_host = match element.value().attr("namehost") {
            Some(name_host) => name_host,
            None => break,
        };
        let link = element.value().attr("link").unwrap();
        let validity = element.value().attr("validity").unwrap();
        links.push(DirectLink::new(name_host.to_string(), link.to_string(), validity.to_string()))
    };
    if links.is_empty() {
        return (ParsedTitle::new(String::new(), 0.0, String::new()), vec![DirectLink::new("error".to_string(), "Invalid link".to_string(), "invalid".to_string())]);
    }
    let mirror_title = website_html.select(&file_name_selector).next().unwrap().next_sibling().unwrap().value().as_text().unwrap().to_string();
    let title_stuff = parse_title(&mirror_title);

    (title_stuff, links)
}

fn parse_title(input: &str) -> ParsedTitle {
    let input = input.trim();
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() >= 3 {
        let file_name = parts[3..parts.len() - 4].join(" ");
        let size = parts[parts.len() - 3].parse::<f64>().unwrap_or(0.0);
        let unit = parts[parts.len() - 2];
        ParsedTitle::new(file_name, size, unit.to_string())
    } else {
        ParsedTitle::new(String::new(), 0.0, String::new())
    }
}

pub async fn get_html(url: &str, client: &Client) -> Result<String, reqwest::Error> {
    //client.get(url).await?.text().await
    client.get(url).send().await?.text().await
}