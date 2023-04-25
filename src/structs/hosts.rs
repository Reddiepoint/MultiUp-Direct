use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Link {
    pub name_host: String,
    pub url: String,
    pub validity: String,
}

impl Link {
    pub fn new(name_host: String, url: String, validity: String) -> Self {
        Link {
            name_host,
            url,
            validity
        }
    }
}

#[derive(Deserialize)]
pub struct LinkValidityResponse {
    error: String,
    file_name: String,
    size: String,
    date_upload: String,
    time_upload: u64,
    date_last_download: String,
    number_downloads: u64,
    description: Option<String>,
    pub hosts: HashMap<String, Option<String>>,
}

#[derive(Serialize)]
pub struct Url {
    pub link: String
}

#[derive(Deserialize)]
pub struct HostInfo {
    selected: String,
    size: u32,
}

#[derive(Deserialize)]
pub struct AvailableHostsResponse {
    error: String,
    pub hosts: HashMap<String, HostInfo>,
    default: Vec<String>,
    #[serde(rename = "maxHosts")]
    max_hosts: u32,
}