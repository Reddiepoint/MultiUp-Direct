use crate::modules::debrid_services::links::{DebridError, DebridServiceLink};
use crossbeam_channel::Sender;
use reqwest::Client;
use serde::Deserialize;
use std::thread;
use tokio::runtime::Runtime;

pub fn unlock_links(
    links: Vec<String>,
    api_key: String,
    link_sender: Sender<Vec<DebridServiceLink>>,
) {
    let rt = Runtime::new().unwrap();
    thread::spawn(move || {
        rt.block_on(async {
            let alldebrid_api_links: Vec<AllDebridDownloadLink> = links
                .iter()
                .map(|link| AllDebridDownloadLink::new(link.to_string(), api_key.clone()))
                .collect();
            let client = Client::new();
            let mut tasks = Vec::new();
            for link in alldebrid_api_links {
                let client = client.clone();
                let task = tokio::spawn(async move {
                    let result = link.unlock(&client).await;
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
                    Err(error) => println!("Error: {}", error),
                }
            }

            let _ = link_sender.send(debrid_links);
        });
    });
}

pub struct AllDebridDownloadLink {
    link: String,
    api_key: String,
}

impl AllDebridDownloadLink {
    fn new(link: String, api_key: String) -> Self {
        Self { link, api_key }
    }

    async fn unlock(&self, client: &Client) -> Result<DebridServiceLink, DebridError> {
        let query = format!(
            "https://api.alldebrid.com/v4/link/unlock?agent=MultiUp-Direct&apikey={}&link={}",
            self.api_key, self.link
        );

        match client.get(query).send().await {
            Ok(response) => match response.json::<AllDebridResponse>().await {
                Ok(response) => Ok(response.new_debrid_service_link(self.link.clone())),
                Err(error) => Err(DebridError::Unknown(error.to_string())),
            },
            Err(error) => Err(DebridError::ReqwestError(error.to_string())),
        }
    }
}
#[derive(Deserialize)]
pub struct AllDebridResponse {
    status: String,
    data: Option<AllDebridData>,
    error: Option<AllDebridError>,
}

impl AllDebridResponse {
    fn new_debrid_service_link(&self, original_link: String) -> DebridServiceLink {
        match &self.data {
            None => {
                if let Some(error) = &self.error {
                    let error = match error.code.as_str() {
                        "AUTH_BAD_APIKEY" => DebridError::BadAPIKey,
                        "LINK_DOWN" => DebridError::LinkDown,
                        "LINK_HOST_UNAVAILABLE" => DebridError::HostDown,
                        "LINK_HOST_NOT_SUPPORTED" => DebridError::HostNotSupported,
                        _ => DebridError::Unknown(error.code.clone()),
                    };
                    DebridServiceLink::new(String::new(), Err(error), original_link)
                } else {
                    panic!("Missing link")
                }
            }
            Some(data) => DebridServiceLink::new(
                data.link.clone(),
                match self.status.as_str() {
                    "success" => Ok(()),
                    _ => Err(DebridError::Unknown(self.status.clone())),
                },
                original_link
            ),
        }
    }
}

#[derive(Deserialize)]
struct AllDebridData {
    link: String,
    // Skip other fields
}

#[derive(Deserialize)]
struct AllDebridError {
    code: String,
    message: String,
}
