use std::collections::BTreeSet;
use std::sync::OnceLock;

use async_recursion::async_recursion;
use crossbeam_channel::{Sender, TryRecvError};
use reqwest::{Client, StatusCode};
use scraper::{Element, Selector};
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
    let link_hosts = scrape_link_for_hosts(&mirror_link.url, client).await;
    if link_hosts.1.first().unwrap().name_host == "error" {
        let url = mirror_link.url.clone();
        let description = link_hosts.1.first().unwrap().url.clone();
        mirror_link.direct_links = Some(BTreeSet::from([DirectLink::new("error".to_string(), format!("{} - {}", description, url), "invalid".to_string())]));
        mirror_link.information = Some(LinkInformation {
            error: "invalid".to_string(),
            file_name: description,
            size: 0.to_string(),
            date_upload: "".to_string(),
            time_upload: 0,
            date_last_download: "N/A".to_string(),
            number_downloads: 0,
            description: Some(url),
            hosts: Default::default(),
        });
        return std::mem::take(mirror_link);
    }
    if !check_status {
        //link_hosts.1.sort_by_key(|link| link.name_host.clone());
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
        return std::mem::take(mirror_link);
    }
    let link_information = check_validity(&mirror_link.url).await;
    let direct_links: BTreeSet<DirectLink> = link_hosts.1.into_iter().map(|link| {
        let status = match link_information.hosts.get(&link.name_host).unwrap() {
            Some(validity) => validity,
            None => "unknown"
        };
        DirectLink::new(link.name_host, link.url, status.to_string())
    }).collect();
    //direct_links.sort_by_key(|link| link.name_host.clone());
    mirror_link.direct_links = Some(direct_links);
    mirror_link.information = Some(link_information);
    std::mem::take(mirror_link)
}

static SELECTOR: OnceLock<Selector> = OnceLock::new();
static FILE_NAME_SELECTOR: OnceLock<Selector> = OnceLock::new();
static QUEUE_SELECTOR: OnceLock<Selector> = OnceLock::new();

#[async_recursion]
async fn scrape_link_for_hosts(url: &str, client: &Client) -> (ParsedTitle, BTreeSet<DirectLink>) {
    // Regular links
    let mut links: BTreeSet<DirectLink> = BTreeSet::new();
    // Scrape panel
    //let now = Instant::now();
    let html = match get_html(url, client).await {
        Ok(html) => html,
        Err(error) => return (ParsedTitle::default(), BTreeSet::from([DirectLink::new("error".to_string(), error.to_string(), "invalid".to_string())]))
    };
    //println!("{}", html);
    //let after = Instant::now();
    //println!("Time taken to load: {}", (after - now).as_millis());

    let selector = SELECTOR.get_or_init(|| Selector::parse(r#"button[type="submit"]"#).unwrap());
    let file_name_selector = FILE_NAME_SELECTOR.get_or_init(|| Selector::parse(r#"body > section > div > section > header > h2 > a"#).unwrap());
    let queue_selector = QUEUE_SELECTOR.get_or_init(|| Selector::parse(r#"body > section > div > section > div.row > div > section > div > div > div:nth-child(2) > div > h4"#).unwrap());

    {
        let website_html = scraper::Html::parse_document(&html);

        for element in website_html.select(queue_selector) {
            let mut queue_status = "";
            for x in element.next_sibling_element().unwrap().text() {
                if x.trim() == "File not found on servers" {
                    queue_status = "File not found on servers";
                    break;
                } else {
                    queue_status = "In queue";
                }
            };
            links.insert(DirectLink::new("error".to_string(), queue_status.to_string(), "invalid".to_string()));
            //if element.next_sibling_element().unwrap().first_child().unwrap() {
            //
            //}
        }
        for element in website_html.select(selector) {
            let element_value = element.value();
            let name_host = match element_value.attr("namehost") {
                Some(name_host) => name_host,
                None => break,
            };
            let link = element_value.attr("link").unwrap();
            let validity = element_value.attr("validity").unwrap();
            links.insert(DirectLink::new(name_host.to_string(), link.to_string(), validity.to_string()));
        };
    }
    if links.is_empty() {
        return scrape_link_for_hosts(url, client).await;
    }

    let website_html = scraper::Html::parse_document(&html);
    let mirror_title = website_html.select(file_name_selector).next().unwrap().next_sibling().unwrap().value().as_text().unwrap().to_string();
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

#[async_recursion]
pub async fn get_html(url: &str, client: &Client) -> Result<String, reqwest::Error> {
    let a = client.get(url).send().await?;
    match a.error_for_status() {
        Ok(res) => res.text().await,
        Err(error) => {
            if error.status().unwrap() != StatusCode::NOT_FOUND {
                return get_html(url, client).await;
            }
            Err(error)
        }
    }
}