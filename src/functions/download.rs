use std::collections::BTreeMap;
use std::sync::mpsc;
use crossbeam_channel::{Sender, SendError};
use eframe::egui::Key::P;
use eframe::egui::TextBuffer;
use once_cell::sync::Lazy;
use reqwest::{Client, Error};
use crate::functions::filter::set_filter_hosts;
use crate::functions::hosts::check_validity;
use crate::structs::hosts::{Link, LinkValidityResponse};


pub async fn generate_direct_links(links: Vec<String>, check_status: bool, number_of_links_tx: Sender<u8>, link_info_tx: Sender<Vec<LinkValidityResponse>>) -> (Vec<Link>, Vec<(String, bool)>) {
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
    let mut direct_links: Vec<Link> = vec![];
    //let mut unordered_links: Vec<(usize, (Vec<Link>, Option<LinkValidityResponse>))> = vec![];
    //for (order, received_links) in rx {
    //    unordered_links.push((order, received_links));
    //    unordered_links.sort_by_key(|(order, link)| order.to_owned());
    //    let ordered_direct_links: Vec<Vec<Link>> = unordered_links.iter().map(|(order, link)| link.0.clone()).collect();
    //    direct_links = ordered_direct_links.iter().flat_map(|link| {
    //        let mut a = link.clone();
    //        a.sort_by_key(|link| link.name_host.clone());
    //        a
    //    }).collect();
    //
    //    let a: Vec<LinkValidityResponse> = unordered_links.iter().filter(|(order, link)| link.1.is_some())
    //        .map(|(order, link)| link.1.clone().unwrap()).collect();
    //    link_info_tx.send(a);
    //    number_of_links_tx.send(1);
    //}

    let mut unordered_links: Vec<(usize, (Vec<Link>, Option<LinkValidityResponse>))> = vec![];
    for (order, received_links) in rx {
        println!("received");

        let mut index = 0;
        while index < unordered_links.len() && unordered_links[index].0 < order {
            index += 1;
        }
        unordered_links.insert(index, (order, received_links));

        // clear the direct_links vector and fill it with the ordered and sorted links

        let responses: Vec<LinkValidityResponse> = unordered_links.iter().filter_map(|(_, (_, response))| response.clone()).collect();
        link_info_tx.send(responses);
        number_of_links_tx.send(1);
    }

    for (_, (links, _)) in unordered_links.iter() {
        let links = links.clone();
        links.clone().sort_by_key(|link| link.name_host.clone());
        direct_links.extend(links);
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

async fn scrape_link(mirror_link: &str, check_status: bool, client: &Client) -> (Vec<Link>, Option<LinkValidityResponse>) {
    let link_hosts = scrape_link_for_hosts(mirror_link, client).await;
    if link_hosts.is_empty() {
        return (vec![Link::new("error".to_string(), "Invalid link".to_string(), "invalid".to_string())], None);
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
        Link::new(link.name_host, link.url, status.to_string())
    }).collect(), Some(hosts))
}

async fn scrape_link_for_hosts(url: &str, client: &Client) -> Vec<Link> {
    // Regular links
    let mut links: Vec<Link> = vec![];
    // Scrape panel
    let website_html = match get_html(url, client).await {
        Ok(html) => html,
        Err(error) => return vec![Link::new("error".to_string(), error.to_string(), "invalid".to_string())]
    };

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

async fn get_html(url: &str, client: &Client) -> Result<String, reqwest::Error> {
    //client.get(url).await?.text().await
    client.get(url).send().await?.text().await
}