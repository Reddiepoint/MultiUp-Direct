
use crate::structs::filter::FilterLinksCriteria;
use crate::structs::hosts::Link;

pub fn filter_links(links: &Vec<Link>, filter: &FilterLinksCriteria) -> String {
    // let display_links: Vec<&str> = links.iter().filter(|link|).map(|link| link.url.as_str()).collect();
    // let display = display_links.join("\n");
    let display_links: Vec<&str> = links
        .iter()
        .filter(|link| match link.validity.as_str() {
            "valid" => filter.valid,
            "invalid" => filter.invalid,
            _ => filter.unknown,
        })
        .filter(|link| {
            filter.hosts.iter().any(|(host_name, selected)| *selected && &link.name_host == host_name)
        })

        .map(|link| {
            link.url.as_str() })
        .collect();

    display_links.join("\n")
}

pub fn get_links_hosts(links: &Vec<Link>) -> Vec<(String, bool)> {
    let mut hosts: Vec<(String, bool)> = vec![];
    for link in links {
        if !hosts.iter().any(|(s, _)| s == &link.name_host) {
            hosts.push((link.name_host.to_string(), false));
        }
    }
    hosts.sort_by_key(|host| host.0.clone());
    hosts
}