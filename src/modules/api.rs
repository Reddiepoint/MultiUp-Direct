use std::collections::{BTreeMap, BTreeSet, HashMap};
use crossbeam_channel::{Receiver, TryRecvError};
use reqwest::{Client, multipart};
use serde::Deserialize;
use crate::modules::debrid::{DebridResponse, DebridService};
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

pub async fn recheck_validity_api(mirror_link: String, mut download_link: DownloadLink, cancel_receiver: Receiver<bool>, client: Client) -> DownloadLink {
    if let Ok(_) | Err(TryRecvError::Disconnected) = cancel_receiver.try_recv() {
        download_link.status = Some(Err(LinkError::Cancelled));
        return download_link;
    }

    // let client = Client::new();
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

#[derive(Clone, Debug, Default, Deserialize)]
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

pub async fn get_fastest_server() -> Result<String, LinkError> {
    let response = match reqwest::get("https://multiup.io/api/get-fastest-server").await {
        Ok(response) => {
            match response.json::<FastestServer>().await {
                Ok(server) => server,
                Err(error) => return Err(LinkError::APIError(error.to_string()))
            }
        },
        Err(error) => return Err(LinkError::Reqwest(error))
    };

    match response.server {
        Some(server) => Ok(server),
        None => Err(LinkError::APIError("No server found".to_string()))
    }
}

#[derive(Debug, Deserialize)]
pub struct MultiUpUploadResponse {
    pub files: Vec<UploadedFileDetails>,
    #[serde(skip)]
    pub project_hash: Option<String>,
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

#[derive(Debug)]
pub struct AddProject {
    pub name: String,
    pub password: Option<String>,
    pub description: Option<String>,
    pub user_id: Option<String>
}

impl AddProject {
    pub fn new(name: String, password: Option<String>, description: Option<String>, user_id: Option<String>) -> Self {
        Self { name, password, description, user_id }
    }

    pub async fn add_project(&self) -> Result<AddProjectResponse, LinkError> {
        let client = Client::new();
        // println!("{:?}", self);
        let mut params = HashMap::new();
        params.insert("name", self.name.clone());
        if let Some(password) = &self.password {
            params.insert("password", password.clone());
        }
        if let Some(description) = &self.description {
            params.insert("description", description.clone());
        }
        if let Some(user_id) = &self.user_id {
            params.insert("user-id", user_id.clone());
        }

        let information = match client.post("https://multiup.io/api/add-project")
            .form(&params)
            .send().await {
            Ok(response) => {
                match response.json::<AddProjectResponse>().await {
                    Ok(information) => information,
                    Err(error) => {
                        return Err(LinkError::APIError(error.to_string()));
                    }
                }
            },
            Err(error) => {
                return Err(LinkError::Reqwest(error));
            }
        };

        Ok(information)
    }
}

#[tokio::test]
async fn test_add_project() {
    let project = AddProject::new("This is a project name".to_string(), Some("123456".to_string()), Some("This is a description".to_string()), Some("1".to_string()));
    match project.add_project().await {
        Ok(response) => { println!("{:?}", response) }
        Err(error) => { eprintln!("{:?}", error) }
    }
}

#[derive(Debug, Deserialize)]
pub struct AddProjectResponse {
    pub error: String,
    pub hash: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub password: Option<String>
}

#[derive(Debug, Default, Deserialize)]
pub struct AvailableHosts {
    pub error: String,
    pub hosts: BTreeMap<String, HostDetails>,
    pub default: Vec<String>,
    #[serde(rename = "maxHosts")]
    pub max_hosts: u32,
}

impl AvailableHosts {
    pub async fn get() -> Result<Self, LinkError> {
        match reqwest::get("https://multiup.io/api/get-list-hosts").await {
            Ok(response) => match response.json::<AvailableHosts>().await {
                Ok(hosts) => Ok(hosts),
                Err(error) => Err(LinkError::APIError(error.to_string())),
            },
            Err(error) => Err(LinkError::Reqwest(error)),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct HostDetails {
    #[serde(rename = "selected")]
    pub selection: String,
    #[serde(skip)]
    pub selected: bool,
    pub size: u64,
}


#[derive(Debug, Deserialize)]
pub struct AllDebridResponse {
    pub status: String,
    pub data: AllDebridData,
}

#[derive(Debug, Deserialize)]
pub struct AllDebridData {
    pub link: String,
    filename: String,
    host: String,
    #[serde(default)]
    streams: Vec<AllDebridStream>,
    streaming: Vec<AllDebridStream>,
    paws: bool,
    filesize: u64,
    id: String,
    #[serde(rename = "hostDomain")]
    host_domain: Option<String>,
    delayed: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct AllDebridStream {
    // pub id: Option<String>,
    // pub ext: Option<String>,
    // pub quality: Option<String>,
    // pub filesize: Option<u64>,
    // pub proto: Option<String>,
    // pub name: Option<String>,
    // pub tb: Option<f64>,
    // pub abr: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct RealDebridResponse {
    id: String,
    filename: String,
    #[serde(rename = "mimeType")]
    mime_type: String,
    filesize: u64,
    #[serde(rename = "link")]
    original_link: String,
    host: String,
    chunks: u32,
    crc: u32,
    #[serde(rename = "download")]
    pub link: String,
    streamable: u32,
}

pub async fn unlock_links(link: &str, debrid_service: DebridService, api_key: &str, client: Client) -> DebridResponse {
    match debrid_service {
        DebridService::AllDebrid => {
            DebridResponse::AllDebrid(unlock_link_with_all_debrid(link, api_key, client).await)
        }
        DebridService::RealDebrid => {
            DebridResponse::RealDebrid(unlock_link_with_real_debrid(link, api_key, client).await)
        }
    }
}

async fn unlock_link_with_all_debrid(link: &str, api_key: &str, client: Client) -> Result<AllDebridResponse, LinkError> {
    let query = format!("https://api.alldebrid.com/v4/link/unlock?agent=MultiUp-Direct&apikey={}&link={}", api_key, link);
    match client.get(query).send().await {
        Ok(response) => match response.json::<AllDebridResponse>().await {
            Ok(debrid_response) => Ok(debrid_response),
            Err(error) => Err(LinkError::APIError(error.to_string())),
        },
        Err(error) => Err(LinkError::Reqwest(error))
    }
}

async fn unlock_link_with_real_debrid(link: &str, api_key: &str, client: Client) -> Result<RealDebridResponse, LinkError> {
    let query = format!("https://api.real-debrid.com/rest/1.0/unrestrict/link?auth_token={}", api_key);
    let mut params = HashMap::new();
    params.insert("link", link);
    // params.insert("remote", "1");
    match client.post(query)
        .form(&params)
        .send().await {
        Ok(response) => {
            match response.json::<RealDebridResponse>().await {
                Ok(debrid_response) => Ok(debrid_response),
                Err(error) => Err(LinkError::APIError(error.to_string()))
            }
        },
        Err(error) => Err(LinkError::Reqwest(error))
    }
}

#[tokio::test]
async fn test_unlock_link_with_real_debrid() {
    let client = Client::new();
    let link = "https://1fichier.com/?y7fuspjxp20btfvpyqt9&af=62851";
    let api_key = "KEY";
    let result = unlock_link_with_real_debrid(link, api_key, client).await;
    println!("{:?}", result);
}