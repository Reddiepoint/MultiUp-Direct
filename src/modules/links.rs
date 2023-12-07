use std::collections::{BTreeSet, HashMap};
use serde::Deserialize;




/// Represents a MultiUp link.
/// Contains the original input link, the fixed mirror link, the extracted direct links
/// and a status reflecting whether the link was successful or not.
pub struct MultiUpLink {
    pub original_link: String,
    pub mirror_link: String,
    pub direct_links: Option<BTreeSet<DirectLink>>,
    pub status: Option<Result<(), String>>
}


/// Represents a direct link within a MultiUp link.
/// Contains the host, URL, validity and whether the link should be displayed in the output.
pub struct DirectLink {
    pub host: String,
    pub url: String,
    pub validity: String,
    pub displayed: String
}

/// Represents information about a MultiUp link from the MultiUp API.
/// Contains details such as the request status, file name, size, upload and download dates,
/// number of downloads, description, and hosts.
///
/// When the API returns an error, only the error field will be returned. Otherwise, it will return
/// `"success"`.
#[derive(Deserialize)]
pub struct MultiUpLinkInformation {
    pub error: String,
    pub file_name: Option<String>,
    pub size: Option<u64>,
    pub date_upload: Option<String>,
    pub time_upload: Option<u64>,
    pub date_last_download: Option<String>,
    pub number_downloads: Option<u64>,
    pub description: Option<String>,
    pub hosts: Option<HashMap<String, String>>,
}

