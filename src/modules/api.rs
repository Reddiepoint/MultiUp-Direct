use std::collections::{BTreeSet, HashMap};
use std::error::Error;
use crossbeam_channel::{Receiver, TryRecvError};
use reqwest::{Client, multipart};
use serde::Deserialize;
use crate::modules::links::{DirectLink, DownloadLink, LinkError};


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

pub async fn recheck_validity_api(mirror_link: String, mut download_link: DownloadLink, cancel_receiver: Receiver<bool>) -> DownloadLink {
    if let Ok(_) | Err(TryRecvError::Disconnected) = cancel_receiver.try_recv() {
        download_link.status = Some(Err(LinkError::Cancelled));
        return download_link;
    }

    let client = Client::new();
    let mut params = HashMap::new();
    params.insert("link", mirror_link);
    let information = match client.post("https://multiup.io/api/check-file")
        .form(&params)
        .send().await {
        Ok(response) => {
            match response.json::<MultiUpLinkInformation>().await {
                Ok(information) => information,
                Err(error) => {
                    download_link.status = Some(Err(LinkError::APIError(error.to_string())));
                    return download_link;
                }
            }
        },
        Err(error) => {
            download_link.status = Some(Err(LinkError::Reqwest(error)));
            return download_link;
        }
    };

    if &information.error != "success" {
        download_link.status = Some(Err(LinkError::APIError(information.error)));
        return download_link;
    }

    let mut new_direct_links = BTreeSet::new();
    if let Some(information) = &information.hosts {
        for (host, validity) in information {
            if let Some(direct_links) = &download_link.direct_links {
                let mut new_direct_link = DirectLink::new(host.clone(), String::new(), validity.clone());
                let original_direct_link = direct_links.get(&new_direct_link);
                if let Some(link) = original_direct_link {
                    new_direct_link.url = link.url.clone();
                }
                new_direct_links.insert(new_direct_link);
            }
        }
    }
    download_link.direct_links = Some(new_direct_links);
    download_link.link_information = Some(information);
    download_link.status = Some(Ok(()));
    download_link
}

#[derive(Clone, Default)]
pub struct Login {
    pub username: String,
    pub password: String,
    pub user_id: String
}

impl Login {
    pub async fn login(&self) -> Result<LoginResponse, LinkError> {
        let client = Client::new();
        let params = multipart::Form::new()
            .text("username", self.username.clone())
            .text("password", self.password.clone());
        match client.post("https://multiup.io/api/login")
            .multipart(params)
            .send().await {
            Ok(response) => {
                match response.json::<LoginResponse>().await {
                    Ok(login_response) => Ok(login_response),
                    Err(error) => {
                        Err(LinkError::APIError(error.to_string()))
                    }
                }
            },
            Err(error) => {
                Err(LinkError::Reqwest(error))
            }
        }
    }
}
#[derive(Default, Deserialize)]
pub struct LoginResponse {
    pub error: String,
    pub login: Option<String>,
    pub user: Option<u64>,
    pub account_type: Option<String>,
    pub premium_days_left: Option<String>
}

#[derive(Deserialize)]
pub struct FastestServer {
    pub error: String,
    pub server: Option<String>,
}

pub async fn get_fastest_server() -> Result<String, Box<dyn Error>> {
    let response = reqwest::get("https://multiup.io/api/get-fastest-server")
        .await?
        .json::<FastestServer>()
        .await?;

    match response.server {
        Some(server) => Ok(server),
        None => Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "No server found")))
    }
}

#[derive(Debug, Deserialize)]
pub struct MultiUpUploadResponse {
    pub files: Vec<UploadedFileDetails>,
}

#[derive(Debug, Deserialize)]
pub struct UploadedFileDetails {
    pub name: Option<String>,
    pub hash: Option<String>,
    pub size: Option<u64>,
    #[serde(rename = "type")]
    pub file_type: Option<String>,
    pub url: Option<String>,
    pub sid: Option<String>,
    pub user: Option<String>,
    pub delete_url: Option<String>,
    pub delete_type: Option<String>,
}