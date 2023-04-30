
use crate::structs::filter::FilterMenu;
use crate::structs::hosts::Link;

pub fn filter_links(links: &[Link], filter: &FilterMenu) -> String {
    // let display_links: Vec<&str> = links.iter().filter(|link|).map(|link| link.url.as_str()).collect();
    // let display = display_links.join("\n");
    let display_links: Vec<&str> = links
        .iter()
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
            link.url.as_str() })
        .collect();

    display_links.join("\n")
}

pub fn set_filter_hosts(links: &[Link]) -> Vec<(String, bool)> {
    let mut hosts: Vec<(String, bool)> = links.iter().map(|link| (link.name_host.to_string(), true)).collect();
    hosts.sort_by_key(|host| host.0.clone());
    hosts.dedup_by_key(|host| host.0.clone());
    hosts

}