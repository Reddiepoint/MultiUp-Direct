use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use eframe::egui::Link;
use headless_chrome::Browser;

use crate::modules::hosting_services::mirrored::links::MirroredLink;
use crate::modules::hosting_services::multiup::links::{MultiUpMirrorLink, MultiUpProjectLink};

/// Represents all the different types of hosting services.
pub enum HostingServiceLink {
    MultiUpProject(MultiUpProjectLink),
    MultiUpMirror(MultiUpMirrorLink),
    Mirrored(MirroredLink),
}

impl HostingService for HostingServiceLink {
    fn get_id(&self) -> &str {
        match self {
            HostingServiceLink::MultiUpProject(link) => link.get_id(),
            HostingServiceLink::MultiUpMirror(link) => link.get_id(),
            HostingServiceLink::Mirrored(link) => link.get_id(),
        }
    }

    fn get_domain(&self) -> &str {
        match self {
            HostingServiceLink::MultiUpProject(link) => link.get_domain(),
            HostingServiceLink::MultiUpMirror(link) => link.get_domain(),
            HostingServiceLink::Mirrored(link) => link.get_domain(),
        }
    }

    /// Unimplemented
    fn get_link_information(&self, browser: &Browser, recheck_validity: bool) -> LinkInformation {
        unimplemented!()
    }
}

impl PartialEq for HostingServiceLink {
    fn eq(&self, other: &Self) -> bool {
        self.get_domain() == other.get_domain() && self.get_id() == other.get_id()
    }
}

/// Represents the consolidated main download link.
#[derive(Debug)]
pub struct LinkInformation {
    pub original_link: String,
    pub domain: String,
    pub id: String,
    pub filename: String,
    pub size: f32,
    pub size_unit: String,
    pub direct_links: Vec<DirectLink>,
    pub status: Result<(), LinkError>,
}

impl PartialEq for LinkInformation {
    fn eq(&self, other: &Self) -> bool {
        self.domain == other.domain && self.filename == other.filename
    }
}

impl LinkInformation {
    pub fn new(
        original_link: String,
        domain: String,
        id: String,
        filename: String,
        size: f32,
        size_unit: String,
        direct_links: Vec<DirectLink>,
        status: Result<(), LinkError>,
    ) -> Self {
        Self {
            original_link,
            domain,
            id,
            filename,
            size,
            size_unit,
            direct_links,
            status,
        }
    }
}

#[derive(Debug)]
pub struct DirectLink {
    pub host: String,
    pub link: String,
    pub validity: LinkValidity,
}

impl DirectLink {
    pub fn new(host: String, link: String, validity: LinkValidity) -> Self {
        Self {
            host,
            link,
            validity,
        }
    }
}

#[derive(Debug)]
pub enum LinkValidity {
    Valid,
    Invalid,
    Unknown,
}

pub trait HostingService {
    fn get_id(&self) -> &str;
    fn get_domain(&self) -> &str;
    fn get_link_information(&self, browser: &Browser, recheck_validity: bool) -> LinkInformation;
}

#[derive(Clone, Debug)]
pub enum LinkError {
    TimedOut(String, u64),
    InvalidLink(String),
    Unknown,
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

impl LinkError {
    pub fn get_error_message(&self) -> String {
        match self {
            LinkError::TimedOut(link, timeout) => {
                format!("{} timed out after {} seconds", link, timeout)
            }
            LinkError::InvalidLink(link) => {
                format!("Invalid link: {}", link)
            }
            LinkError::Unknown => {
                "Unknown error".to_string()
            }
        }
    }
}