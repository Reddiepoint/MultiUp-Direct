use crate::modules::config;
use crate::modules::config::{Config, ConfigError};

#[derive(Default, PartialEq)]
pub enum DebridService {
    #[default]
    AllDebrid,
    RealDebrid,
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
    pub all_debrid: String,
    pub real_debrid: String,
}

impl Default for DebridAPIKeys {
    fn default() -> Self {
        let config = config::read_config().unwrap_or_default();
        let mut debrid_api_keys = DebridAPIKeys {
            all_debrid: String::new(),
            real_debrid: String::new(),
        };
        if let Some(key) = config.all_debrid_api_key {
            debrid_api_keys.all_debrid = key;
        } else if let Some(key) = config.real_debrid_api_key {
            debrid_api_keys.real_debrid = key;
        }
        debrid_api_keys
    }
}