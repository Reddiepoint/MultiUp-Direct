use crate::modules::config;
use crate::modules::config::{Config, ConfigError};
use crate::modules::debrid_services::links::recognise_links;

#[derive(PartialEq)]
pub enum DebridService {
    AllDebrid,
    RealDebrid,
}

impl DebridService {
    pub fn unlock_links(&self, links: &str) {
        let links = recognise_links(links);
        match self {
            DebridService::AllDebrid => {}
            DebridService::RealDebrid => {}
        }
    }
}

impl Default for DebridService {
    fn default() -> Self {
        let config = config::read_config().unwrap_or_default();
        if let Some(key) = config.alldebrid_api_key {
            DebridService::AllDebrid
        } else if let Some(key) = config.real_debrid_api_key {
            DebridService::RealDebrid
        } else {
            DebridService::AllDebrid
        }
    }
}
impl DebridService {
    pub fn supports_remote_traffic(&self) -> bool {
        match self {
            DebridService::AllDebrid => false,
            DebridService::RealDebrid => true,
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