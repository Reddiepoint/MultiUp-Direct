use crate::modules::config;
use crate::modules::debrid_services::alldebrid;
use crate::modules::debrid_services::links::{recognise_links, DebridServiceLink};
use crossbeam_channel::Sender;

#[derive(PartialEq)]
pub enum DebridService {
    AllDebrid(String),
    RealDebrid(String),
}

impl DebridService {
    pub fn unlock_links(&self, links: &str, link_sender: Sender<Vec<DebridServiceLink>>) {
        let links = recognise_links(links);
        match self {
            DebridService::AllDebrid(api_key) => {
                alldebrid::unlock_links(links, api_key.clone(), link_sender)
            }
            DebridService::RealDebrid(api_key) => {}
        }
    }
}

impl Default for DebridService {
    fn default() -> Self {
        let config = config::read_config().unwrap_or_default();
        if let Some(key) = config.alldebrid_api_key {
            DebridService::AllDebrid(key)
        } else if let Some(key) = config.real_debrid_api_key {
            DebridService::RealDebrid(key)
        } else {
            DebridService::AllDebrid(String::new())
        }
    }
}
impl DebridService {
    pub fn supports_remote_traffic(&self) -> bool {
        match self {
            DebridService::AllDebrid(_) => false,
            DebridService::RealDebrid(_) => true,
        }
    }
}

pub struct DebridAPIKeys {
    pub alldebrid: String,
    pub real_debrid: String,
}

impl Default for DebridAPIKeys {
    fn default() -> Self {
        let config = config::read_config().unwrap_or_default();
        let mut debrid_api_keys = DebridAPIKeys {
            alldebrid: String::new(),
            real_debrid: String::new(),
        };
        if let Some(key) = config.alldebrid_api_key {
            debrid_api_keys.alldebrid = key;
        } else if let Some(key) = config.real_debrid_api_key {
            debrid_api_keys.real_debrid = key;
        }
        debrid_api_keys
    }
}
