use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};

use async_recursion::async_recursion;
use serde::{Deserialize, Serialize};



#[derive(Clone, Debug, Eq)]
pub struct DirectLink {
    pub name_host: String,
    pub url: String,
    pub validity: String,
    pub displayed: bool,
}

impl PartialOrd for DirectLink {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.name_host.cmp(&other.name_host))
    }
}

impl Ord for DirectLink {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name_host.cmp(&other.name_host)
    }
}

impl PartialEq for DirectLink {
    fn eq(&self, other: &Self) -> bool {
        self.name_host == other.name_host
    }
}

impl DirectLink {
    pub fn new(name_host: String, url: String, validity: String) -> Self {
        DirectLink {
            name_host,
            url,
            validity,
            displayed: false,
        }
    }
}

/// Contains the fixed mirror link and possibly the generated direct links
#[derive(Default, Clone)]
pub struct MirrorLink {
    pub url: String,
    pub direct_links: Option<BTreeSet<DirectLink>>,
    pub information: Option<LinkInformation>,
}

impl PartialEq for MirrorLink {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
    }
}

impl MirrorLink {
    pub fn new(url: String) -> Self {
        Self {
            url,
            direct_links: None,
            information: None,
        }
    }
}

#[derive(Clone, Deserialize)]
pub struct LinkInformation {
    pub error: String,
    pub file_name: String,
    pub size: String,
    pub date_upload: String,
    pub time_upload: u64,
    pub date_last_download: String,
    pub number_downloads: u64,
    pub description: Option<String>,
    pub hosts: HashMap<String, Option<String>>,
}

#[derive(Serialize)]
pub struct Url {
    pub link: String,
}

#[async_recursion]
pub async fn check_validity(url: &str) -> LinkInformation {
    let client = reqwest::Client::new();
    match client.post("https://multiup.org/api/check-file")
        .form(&Url { link: url.to_string() })
        .send().await.unwrap().json::<LinkInformation>().await {
        Ok(information) => information,
        Err(_error) => {
            LinkInformation {
                error: "error".to_string(),
                file_name: "File not available".to_string(),
                size: "0".to_string(),
                date_upload: "0".to_string(),
                time_upload: 0,
                date_last_download: "0".to_string(),
                number_downloads: 0,
                description: Some(url.to_string()),
                hosts: Default::default(),
            }
        }
    }
}

//pub fn get_available_hosts() -> Vec<String> {
//    let rt = Runtime::new().expect("Unable to create runtime");
//    let _ = rt.enter();
//    let (tx, rx) = mpsc::sync_channel(0);
//    std::thread::spawn(move || {
//        let host_list = rt.block_on(async {
//            let response = reqwest::get("https://multiup.org/api/get-list-hosts").await.unwrap().json::<AvailableHostsResponse>().await.unwrap();
//            let mut list = vec![];
//            for (i, _j) in response.hosts {
//                list.push(i);
//            };
//            list
//        });
//        tx.send(host_list)
//    });
//
//    rx.recv().unwrap()
//}


