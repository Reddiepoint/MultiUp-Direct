use std::collections::HashMap;
use crate::modules::debrid_services::links::{DebridError, DebridServiceLink};
use crossbeam_channel::Sender;
use reqwest::Client;
use serde::Deserialize;
use std::thread;
use reqwest::header::AUTHORIZATION;
use tokio::runtime::Runtime;

pub fn unlock_links(
    links: Vec<String>,
    api_key: String,
    use_remote_traffic: bool,
    link_sender: Sender<Vec<DebridServiceLink>>,
) {
    let rt = Runtime::new().unwrap();
    thread::spawn(move || {
        rt.block_on(async {
            let real_debrid_api_links: Vec<RealDebridDownloadLink> = links
                .iter()
                .map(|link| RealDebridDownloadLink::new(link.to_string(), api_key.clone()))
                .collect();
            let client = Client::new();
            let mut tasks = Vec::new();
            for link in real_debrid_api_links {
                let client = client.clone();
                let task = tokio::spawn(async move {
                    let result = link.unlock(&client, use_remote_traffic).await;
                    result
                });
                tasks.push(task);
            }

            let mut debrid_links: Vec<DebridServiceLink> = Vec::new();
            let results = futures::future::join_all(tasks).await;
            for result in results {
                match result {
                    Ok(debrid_link) => {
                        if let Ok(link) = debrid_link {
                            debrid_links.push(link);
                        }
                    }
                    Err(error) => eprintln!("Error: {}", error),
                }
            }

            let _ = link_sender.send(debrid_links);
        });
    });
}

pub struct RealDebridDownloadLink {
    link: String,
    api_key: String,
}
impl RealDebridDownloadLink {
    fn new(link: String, api_key: String) -> Self {
        Self { link, api_key }
    }

    async fn unlock(&self, client: &Client, use_remote_traffic: bool) -> Result<DebridServiceLink, DebridError> {
        let query = "https://api.real-debrid.com/rest/1.0/unrestrict/link";
        let mut params = HashMap::new();
        params.insert("link", self.link.as_str());
        params.insert("remote", match use_remote_traffic {
            true => "1",
            false => "0",
        });
        match client.post(query)
            .header("Authorization", format!("Bearer {}", self.api_key.clone()))
            .form(&params)
            .send().await {
            Ok(response) => match response.json::<RealDebridResponse>().await {
                Ok(response) => Ok(response.new_debrid_service_link(self.link.clone())),
                Err(error) => Err(DebridError::Unknown(error.to_string())),
            },
            Err(error) => Err(DebridError::ReqwestError(error.to_string())),
        }
    }
}

#[derive(Deserialize)]
pub struct RealDebridResponse {
    download: Option<String>,
    error: Option<String>,
    error_details: Option<String>
}

impl RealDebridResponse {
    fn new_debrid_service_link(&self, original_link: String) -> DebridServiceLink {
        match &self.error {
            Some(error) => {
                let error = match error.as_str() {
                    "hoster_not_free" | "hoster_unsupported" => DebridError::HostNotSupported,
                    _ => DebridError::Unknown(self.error_details.clone().unwrap_or_default())
                };
                DebridServiceLink::new(String::new(), Err(error), original_link)
            }
            None => {
                DebridServiceLink::new(
                    self.download.clone().unwrap_or_default(),
                    Ok(()),
                    original_link,
                )
            }

        }
    }
}
