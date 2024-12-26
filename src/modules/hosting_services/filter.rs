use crate::modules::hosting_services::links::{DirectLinkValidity, LinkInformation};
use std::collections::BTreeMap;

pub fn get_info(links: &Vec<LinkInformation>) -> (u32, u32, u32, BTreeMap<String, u32>) {
    let mut valid: u32 = 0;
    let mut invalid: u32 = 0;
    let mut unknown: u32 = 0;
    let mut hosts: BTreeMap<String, u32> = BTreeMap::new();

    for link in links {
        for direct_link in &link.direct_links {
            match direct_link.validity {
                DirectLinkValidity::Valid => valid += 1,
                DirectLinkValidity::Invalid => invalid += 1,
                DirectLinkValidity::Unknown => unknown += 1,
            }

            hosts
                .entry(direct_link.host.clone())
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }
    }

    (valid, invalid, unknown, hosts)
}

pub fn count_hosts(hosts: &Vec<String>) -> BTreeMap<String, u32> {
    let mut count: BTreeMap<String, u32> = BTreeMap::new();
    for host in hosts {
        count
            .entry(host.clone())
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }
    count
}
