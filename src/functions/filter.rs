use std::collections::BTreeSet;
use crate::structs::filter::FilterMenu;
use crate::structs::hosts::DirectLink;

pub fn filter_links(links: &[DirectLink], filter: &FilterMenu) -> Vec<(bool, String)> {
    links
        .iter()
        .filter(|link| link.displayed)
        .filter(|link| match link.validity.as_str() {
            "valid" => filter.valid,
            "invalid" => filter.invalid,
            "unknown" => filter.unknown,
            _ => filter.unchecked,
        })
        .filter(|link| {
            filter.hosts.iter().any(|(host_name, selected)| *selected && &link.name_host == host_name)
        })
        .map(|link| {
            (link.displayed, link.url.to_string()) })
        .collect()
}

pub fn set_filter_hosts(links: &[DirectLink]) -> Vec<(String, bool)> {
    let mut hosts: BTreeSet<String> = BTreeSet::new();
    for link in links {
        hosts.insert(link.name_host.to_string());
    }

    hosts.into_iter().map(|host| (host, true)).collect()
}