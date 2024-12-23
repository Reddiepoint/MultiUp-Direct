use std::fmt::Display;
use regex::Regex;

pub struct DebridServiceLink {
    pub link: String,
    pub status: Result<(), DebridError>,
    pub original_link: String,
}

impl DebridServiceLink {
    pub fn new(link: String, status: Result<(), DebridError>, original_link: String) -> Self {
        DebridServiceLink { link, status, original_link }
    }
}

#[derive(Clone)]
pub enum DebridError {
    ReqwestError(String),
    HostNotSupported,
    HostDown,
    LinkDown,
    Unknown(String),
}

impl DebridError {
    pub fn get_error_message(&self) -> String {
        match self {
            DebridError::ReqwestError(error) => {
                format!("Reqwest error: {}", error)
            }
            DebridError::HostNotSupported => {
                "Host not supported".to_string()
            }
            DebridError::HostDown => {
                "Host down or under maintenance".to_string()
            }
            DebridError::LinkDown => {
                "Link offline".to_string()
            }
            DebridError::Unknown(error) => {
                format!("Unknown error: {}", error)
            }
        }
    }
}

pub fn recognise_links(raw_links: &str) -> Vec<String> {
    let generic_link_regex = Regex::new(r"(?:https?://)?(?:www\.)?([-a-zA-Z0-9@%._+~#=]{2,256}\.[a-z]{2,6})/([-a-zA-Z0-9@:%_+.~#?&/=]*)").unwrap();
    let mut detected_links = Vec::new();
    for (link, [domain, slug]) in generic_link_regex
        .captures_iter(raw_links)
        .map(|c| c.extract()) {
        detected_links.push(link.to_owned());
    }
    detected_links
}

