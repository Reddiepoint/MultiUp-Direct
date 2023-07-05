use crossbeam_channel::{Sender};
use std::sync::OnceLock;
use reqwest::{Client};
use crate::functions::hosts::check_validity;
use crate::structs::download::ParsedTitle;
use crate::structs::hosts::{DirectLink, LinkInformation, MirrorLink};

static MULTIUP_REGEX: OnceLock<regex::Regex> = OnceLock::new();
/// Convert short and long links to the en/mirror page. Removes duplicates
pub fn fix_multiup_links(multiup_links: &str) -> Vec<MirrorLink> {
    let mut mirror_links = Vec::with_capacity(multiup_links.lines().count()); // Pre-allocate memory for the vector
    //let mut mirror_links = vec![];
    let mirror_prefix = "https://multiup.org/en/mirror/";
    let multiup_regex = MULTIUP_REGEX.get_or_init(|| regex::Regex::new(r#"^https?://multiup\.org/en/mirror/[^/]+/[^/]+$"#).unwrap());
    for line in multiup_links.lines() {
        let multiup_link = line.trim().split(' ').next().unwrap();
        let multiup_link = multiup_link.replace("www.", ""); // Compatibility for older links

        let prefixes = ["https://multiup.org/download/", "http://multiup.org/download/", "https://multiup.org/en/download/", "https://multiup.org/"];
        let fixed_link = if multiup_regex.is_match(&multiup_link) {
            multiup_link
        } else {
            let mut mirror_link = String::new();
            for prefix in prefixes {
                if let Some(suffix) = multiup_link.strip_prefix(prefix) {
                    let mut fixed_link = format!("{}{}", mirror_prefix, suffix);
                    if !multiup_regex.is_match(&fixed_link) {
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
    let mut link_hosts = scrape_link_for_hosts(&mirror_link.url, client).await;
    if link_hosts.1.is_empty() {
        mirror_link.direct_links = Some(vec![DirectLink::new("error".to_string(), "Invalid link".to_string(), "invalid".to_string())]);
        return mirror_link.clone()
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

static SELECTOR: OnceLock<scraper::Selector> = OnceLock::new();
static FILE_NAME_SELECTOR: OnceLock<scraper::Selector> = OnceLock::new();
async fn scrape_link_for_hosts(url: &str, client: &Client) -> (ParsedTitle, Vec<DirectLink>) {
    // Regular links
    let mut links: Vec<DirectLink> = vec![];
    // Scrape panel
    let website_html = match get_html(url, client).await {
        Ok(html) => html,
        Err(error) => return (ParsedTitle::new(String::new(), 0.0, String::new()), vec![DirectLink::new("error".to_string(), error.to_string(), "invalid".to_string())])
    };
    let selector = SELECTOR.get_or_init(|| scraper::Selector::parse(r#"button[type="submit"]"#).unwrap());
    let file_name_selector = FILE_NAME_SELECTOR.get_or_init(|| scraper::Selector::parse(r#"body > section > div > section > header > h2 > a"#).unwrap());
    let website_html = scraper::Html::parse_document(&website_html);
    for element in website_html.select(&selector) {
        let name_host = element.value().attr("namehost").unwrap();
        let link = element.value().attr("link").unwrap();
        let validity = element.value().attr("validity").unwrap();
        links.push(DirectLink::new(name_host.to_string(), link.to_string(), validity.to_string()))
    };
    let mirror_title = website_html.select(&file_name_selector).next().unwrap().next_sibling().unwrap().value().as_text().unwrap().to_string();
    let title_stuff = parse_title(&mirror_title);

    (title_stuff, links) // Will be empty if invalid page
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