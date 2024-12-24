use headless_chrome::Browser;
use std::fmt::{Debug, Display, Formatter};

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
    pub validity: DirectLinkValidity,
}

impl DirectLink {
    pub fn new(host: String, link: String, validity: DirectLinkValidity) -> Self {
        Self {
            host,
            link,
            validity,
        }
    }
}

#[derive(Debug)]
pub enum DirectLinkValidity {
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
    BrowserError(String),
    TimedOut(String, u64),
    InvalidLink(String),
    Unknown(String),
}

impl Display for LinkError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkError::TimedOut(error, timeout) => {
                write!(f, "Timed out after {} seconds: {}", timeout, error)
            }
            LinkError::InvalidLink(error) => {
                write!(f, "Invalid link: {}", error)
            }
            LinkError::Unknown(error) => {
                write!(f, "Unknown error: {}", error)
            }
            LinkError::BrowserError(error) => {
                write!(f, "Browser error: {}", error)
            }
        }
    }
}

impl LinkError {
    pub fn get_error_message(&self) -> String {
        match self {
            LinkError::TimedOut(error, timeout) => {
                format!("Timed out after {} seconds: {}", timeout, error)
            }
            LinkError::InvalidLink(error) => {
                format!("Invalid link: {}", error)
            }
            LinkError::Unknown(error) => {
                format!("Unknown error: {}", error)
            }
            LinkError::BrowserError(error) => {
                format!("Browser error: {}", error)
            }
        }
    }
}
