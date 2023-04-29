use std::sync::mpsc;
use eframe::egui::{ScrollArea, TextEdit, Ui};
use tokio::runtime::Runtime;
use crate::functions::filter::get_links_hosts;
use crate::functions::hosts::check_validity;
use crate::structs::download::Download;
use crate::structs::hosts::Link;


pub async fn generate_direct_links(links: &str, check_status: &bool, filter: &Vec<(String, bool)>) -> (Vec<Link>, Vec<(String, bool)>) {
	let links = fix_mirror_links(&links); // All links are ok

	let direct_links = scrape_links(&links, check_status).await;
	println!("{:?}", direct_links);
	let links_hosts = get_links_hosts(&direct_links);
	(direct_links, links_hosts)
}


pub fn fix_mirror_links(url: &str) -> Vec<String> {
	let re = regex::Regex::new(r"^https://multiup\.org/en/mirror/[^/]+/[^/]+$").unwrap();
	url
		.split('\n')
		.map(|link| {
			let mut link = link.trim().split(' ').next().unwrap();
			let fixed_link = if (link.starts_with("https://multiup.org/") || link.starts_with("https://multiup.org/download/")) && !link.starts_with("https://multiup.org/en/mirror/") {
				let mut fixed_link = link.replace("https://multiup.org/", "https://multiup.org/en/mirror/").replace("download/", "");
				if !re.is_match(&fixed_link) {
					fixed_link += "/a";
					fixed_link
				} else {
					fixed_link
				}
			} else {
				link.to_string()
			};

			if !re.is_match(&fixed_link) {
				String::new()
			} else {
				fixed_link
			}
		})
		.filter(|link| !link.is_empty())
		.collect::<Vec<String>>()
}

async fn scrape_links(mirror_links: &[String], check_status: &bool) -> Vec<Link> {
	let mut direct_links = vec![];
	for link in mirror_links {
		let mut link_hosts = scrape_link_hosts(link).await;
		if link_hosts.is_empty() {
			direct_links.push(Link::new("error".to_string(), "Invalid link".to_string(), "invalid".to_string()));
		} else if *check_status {
			let rt = Runtime::new().expect("Unable to create runtime");
			let temp_link = link.clone();
			let (tx, rx) = mpsc::sync_channel(0);
			std::thread::spawn(move || {
				let hosts = rt.block_on(async { check_validity(&temp_link).await });
				tx.send(hosts)
			});
			let hosts = rx.recv().unwrap();
			link_hosts = link_hosts
				.into_iter()
				.map(|direct_link| {
					if let Some(status) = hosts.get(&direct_link.name_host) {
						match status {
							Some(status) => Link::new(direct_link.name_host, direct_link.url, status.to_string()),
							None => Link::new(direct_link.name_host, direct_link.url, String::new()),
						}
					} else {
						direct_link
					}
				})
				.collect();
		}
		direct_links.append(&mut link_hosts);
	}
	direct_links
}

async fn scrape_link_hosts(url: &str) -> Vec<Link> {
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