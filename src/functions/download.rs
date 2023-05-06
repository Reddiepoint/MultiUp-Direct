use std::sync::mpsc;
use crate::functions::filter::set_filter_hosts;
use crate::functions::hosts::check_validity;
use crate::structs::hosts::Link;


pub async fn generate_direct_links(links: &str, check_status: bool) -> (Vec<Link>, Vec<(String, bool)>) {
    let links = fix_mirror_links(links); // All links are ok

    let (tx, rx) = mpsc::channel();
    let mut n = 1;
    for link in links {
        let tx = tx.clone();
        let temp_link = link.clone();
        tokio::spawn(async move {
            let generated_links = scrape_link(&temp_link, check_status).await;
            tx.send((generated_links, n)).unwrap();
        });
        n += 1
    }
    drop(tx);
    //let mut direct_links: Vec<Link> = vec![];
    //for (received_links, order) in rx {
    //    direct_links.extend(received_links);
    //}
    let a = vec![1];

    let mut ordered_links: Vec<(Vec<Link>, i32)> = vec![];
    for (mut a, b) in rx {
        a.sort_by_key(|link| link.name_host.clone());
        ordered_links.push((a, b));
    }

    ordered_links.sort_by_key(|a| a.1);
    let direct_links: Vec<Link> = ordered_links.into_iter().flat_map(|v| v.0).collect();

    let filter_hosts = set_filter_hosts(&direct_links);
    (direct_links, filter_hosts)
}

/// Convert short and long links to the en/mirror page. Removes duplicates
pub fn fix_mirror_links(links: &str) -> Vec<String> {
    let re = regex::Regex::new(r"^https://multiup\.org/en/mirror/[^/]+/[^/]+$").unwrap();
    let a: Vec<String> = links.split('\n')
        .map(|link| {
            let link = link.trim().split(' ').next().unwrap();
            let fixed_link = if !link.starts_with("https://multiup.org/en/mirror/") && link.starts_with("https://multiup.org/") {
                let mut fixed_link = link.replace("https://multiup.org/", "https://multiup.org/en/mirror/").replace("download/", "");
                if !re.is_match(&fixed_link) {
                    fixed_link += "/a";
                };
                fixed_link
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
        .collect();

    let mut mirror_links = vec![];
    for link in a {
        if !mirror_links.contains(&link) {
            mirror_links.push(link)
        }
    };
    mirror_links
}


async fn scrape_link(mirror_link: &str, check_status: bool) -> Vec<Link> {
    let link_hosts = scrape_link_for_hosts(mirror_link).await;
    if link_hosts.is_empty() {
        vec![Link::new("error".to_string(), "Invalid link".to_string(), "invalid".to_string())]
    } else if check_status {
        let hosts = check_validity(mirror_link).await;
        link_hosts.into_iter().map(|link| {
            let status = match hosts.get(&link.name_host).unwrap() {
                Some(validity) => validity,
                None => "unknown"
            };
            Link::new(link.name_host, link.url, status.to_string())
        }).collect()
    } else {
        link_hosts
    }
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