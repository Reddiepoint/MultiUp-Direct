use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct DirectLink {
    pub name_host: String,
    pub url: String,
    pub validity: String,
    pub displayed: bool
}

impl DirectLink {
    pub fn new(name_host: String, url: String, validity: String) -> Self {
        DirectLink {
            name_host,
            url,
            validity,
            displayed: false
        }
    }
}

/// Contains the fixed mirror link and possibly the generated direct links
#[derive(Clone)]
pub struct MirrorLink {
    pub url: String,
    pub direct_links: Option<Vec<DirectLink>>,
    pub information: Option<LinkInformation>
}

impl MirrorLink {
    pub fn new(url: String) -> Self {
        Self {
            url,
            direct_links: None,
            information: None
        }
    }
}

#[derive(Clone, Deserialize)]
pub struct LinkInformation {
    pub error: String,
    pub file_name: String,
    pub size: String,
    pub date_upload: String,
    pub time_upload: u64,
    pub date_last_download: String,
    pub number_downloads: u64,
    pub description: Option<String>,
    pub hosts: HashMap<String, Option<String>>,
}

#[derive(Serialize)]
pub struct Url {
    pub link: String
}

//#[derive(Deserialize)]
//pub struct HostInfo {
//    selected: String,
//    size: u32,
//}
//
//#[derive(Deserialize)]
//pub struct AvailableHostsResponse {
//    error: String,
//    pub hosts: HashMap<String, HostInfo>,
//    default: Vec<String>,
//    #[serde(rename = "maxHosts")]
//    max_hosts: u32,
//}