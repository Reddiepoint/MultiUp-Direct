use std::collections::{BTreeSet, HashMap};
use crossbeam_channel::{Receiver, TryRecvError};
use reqwest::Client;
use crate::modules::links::{DirectLink, DownloadLink, LinkError, MultiUpLinkInformation};

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