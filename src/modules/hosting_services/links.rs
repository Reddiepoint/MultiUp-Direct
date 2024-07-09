use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::io::ErrorKind;
use headless_chrome::Browser;
use crate::modules::hosting_services::multiup::links::{MultiUpMirrorLink, MultiUpProjectLink};

/// Represents all the different types of hosting services.
pub enum HostingServiceLink {
    MultiUpProject(MultiUpProjectLink),
    MultiUpMirror(MultiUpMirrorLink),
    
}


/// Represents the consolidated main download link.
pub struct DownloadLink {
    original_link: String,
    size: f32,
    size_unit: String,
    direct_links: Vec<DirectLink>,
}
pub struct DirectLink {
    
}

pub trait HostingService {
    fn is_project(&self) -> bool {
        false
    }
    fn get_direct_links(&self, browser: Browser) -> Vec<DirectLink>;
}

#[derive(Debug)]
pub enum LinkError {
    TimedOut(String, u64),
    InvalidLink(String),
    Unknown
}

impl Display for LinkError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkError::TimedOut(link, timeout) => {
                write!(f, "{} timed out after {} seconds", link, timeout)
            }
            LinkError::InvalidLink(link) => {
                write!(f, "Invalid link: {}", link)
            }
            LinkError::Unknown => {
                write!(f, "Unknown error")
            }
        }
    }
}

impl Error for LinkError {
    
}