use serde::{Deserialize, Serialize};
use std::io::ErrorKind;

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub check_for_updates_on_startup: bool,
    pub alldebrid_api_key: Option<String>,
    pub real_debrid_api_key: Option<String>,
    pub hosting_passwords: Vec<String>
}

impl Default for Config {
    fn default() -> Self {
        Self {
            check_for_updates_on_startup: true,
            alldebrid_api_key: None,
            real_debrid_api_key: None,
            hosting_passwords: Vec::new(),
        }
    }
}

pub enum ConfigError {
    ConfigNotFound,
    InvalidJson,
    Unknown(String),
}

impl ConfigError {
    pub fn get_error_message(&self) -> String {
        match self {
            ConfigError::ConfigNotFound => {
                "Config file not found.".to_string()
            }
            ConfigError::InvalidJson => {
                "Invalid JSON in config file.".to_string()
            }
            ConfigError::Unknown(error) => {
                format!("Unknown error: {}", error)
            }
        }
    }
}

pub fn read_config() -> Result<Config, ConfigError> {
    match std::fs::read_to_string("config.json") {
        Ok(config) => match serde_json::from_str(&config) {
            Ok(config) => Ok(config),
            Err(error) => {
                eprintln!("Error reading config: {}", error);
                Err(ConfigError::InvalidJson)
            }
        },
        Err(error) => match error.kind() {
            ErrorKind::NotFound => Err(ConfigError::ConfigNotFound),
            _ => Err(ConfigError::Unknown(error.to_string())),
        },
    }
}

pub fn save_config(config: &Config) -> Result<(), ConfigError> {
    let json = serde_json::to_string_pretty(config).unwrap();
    if let Err(error) = std::fs::write("config.json", json) {
        return Err(ConfigError::Unknown(error.to_string()));
    };
    Ok(())
}
