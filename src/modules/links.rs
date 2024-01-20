use std::cmp::Ordering;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};

#[derive(Debug)]
pub enum MultiUpLink {
    Project(ProjectLink),
    Download(DownloadLink),
}

impl PartialEq for MultiUpLink {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Project(project_link_l), Self::Project(project_link_r)) => {
                project_link_l == project_link_r
            }
            (Self::Download(download_link_l), Self::Download(download_link_r)) => {
                download_link_l == download_link_r
            }
            _ => false,
        }
    }
}


/// Represents a MultiUp project link.
/// Contains the original input link, the ID of the link,
/// the name of the project, the extracted download links,
/// and a status reflecting whether the link was successful or not.
#[derive(Debug)]
pub struct ProjectLink {
    pub original_link: String,
    pub link_id: String,
    pub name: String,
    pub download_links: Option<HashSet<DownloadLink>>,
    pub status: Option<Result<(), LinkError>>,
}

// Compares link_id
impl PartialEq for ProjectLink {
    fn eq(&self, other: &Self) -> bool {
        self.link_id == other.link_id
    }
}

impl ProjectLink {
    pub fn new(original_link: String, link_id: String, name: String) -> Self {
        Self {
            original_link,
            link_id,
            name,
            download_links: None,
            status: None,
        }
    }
}


/// Represents a MultiUp download link.
/// Contains the original input link, the ID of the link, the extracted direct links
/// and a status reflecting whether the link was successful or not.
#[derive(Debug)]
pub struct DownloadLink {
    pub original_link: String,
    pub link_id: String,
    pub direct_links: Option<BTreeSet<DirectLink>>,
    pub link_information: Option<MultiUpLinkInformation>,
    pub status: Option<Result<(), LinkError>>,
}

impl Hash for DownloadLink {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.link_id.hash(state);
    }
}

impl PartialEq for DownloadLink {
    fn eq(&self, other: &Self) -> bool {
        self.link_id == other.link_id
    }
}

impl Eq for DownloadLink {}

impl DownloadLink {
    pub fn new(original_link: String, link_id: String) -> Self {
        Self {
            original_link,
            link_id,
            direct_links: None,
            link_information: None,
            status: None,
        }
    }
}

/// Represents a direct link within a MultiUp link.
/// Contains the host, URL, validity and whether the link should be displayed in the output.
#[derive(Debug)]
pub struct DirectLink {
    pub host: String,
    pub url: String,
    pub validity: String,
    pub displayed: bool,
}

impl PartialEq for DirectLink {
    fn eq(&self, other: &Self) -> bool {
        self.host == other.host
    }
}

impl Eq for DirectLink {}

impl PartialOrd for DirectLink {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.host.cmp(&other.host))
    }
}

impl Ord for DirectLink {
    fn cmp(&self, other: &Self) -> Ordering {
        self.host.cmp(&other.host)
    }
}

impl DirectLink {
    pub fn new(host: String, url: String, validity: String) -> Self {
        Self {
            host,
            url,
            validity,
            displayed: true,
        }
    }
}

/// Represents information about a MultiUp link from the MultiUp API.
/// Contains details such as the request status, file name, size (in bytes), upload and download dates,
/// number of downloads, description, and hosts.
///
/// When the API returns an error, only the error field will be returned. Otherwise, it will return
/// `"success"`.
#[derive(Debug, Deserialize)]
pub struct MultiUpLinkInformation {
    pub error: String,
    pub file_name: Option<String>,
    pub size: Option<String>,
    pub date_upload: Option<String>,
    pub time_upload: Option<u64>,
    pub date_last_download: Option<String>,
    pub number_downloads: Option<u64>,
    pub description: Option<String>,
    pub hosts: Option<HashMap<String, String>>,
}

impl MultiUpLinkInformation {
    pub fn new_basic(file_name: String, size: String) -> Self {
        Self {
            error: "success".to_string(),
            file_name: Some(file_name),
            size: Some(size),
            date_upload: None,
            time_upload: None,
            date_last_download: None,
            number_downloads: None,
            description: None,
            hosts: None,
        }
    }
}
#[derive(Debug)]
pub enum LinkError {
    APIError(String),
    Cancelled,
    Invalid,
    InQueue,
    NoLinks,
    Other,
    Reqwest(reqwest::Error),
    TimedOut
}
