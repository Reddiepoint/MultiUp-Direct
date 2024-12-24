use crate::modules::hosting_services::browser::new_browser;
use crate::modules::hosting_services::links::{
    HostingService, HostingServiceLink, LinkInformation,
};
use crate::modules::hosting_services::mirrored::links::new_mirrored_link;
use crate::modules::hosting_services::multiup::links::new_multiup_link;
use crossbeam_channel::Sender;
use regex::Regex;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::thread;
use tokio::runtime::Runtime;
use tokio::sync::Semaphore;

pub fn extract_raw_links(
    raw_hosting_service_links: String,
    recheck_validity: bool,
    link_information_sender: Sender<Vec<LinkInformation>>,
) {
    let hosting_service_links = recognise_links(&raw_hosting_service_links);
    let rt = Runtime::new().unwrap();
    thread::spawn(move || {
        rt.block_on(async {
            let download_links = extract_download_links(
                &hosting_service_links,
                recheck_validity, /*, cancel_receiver*/
            )
            .await;
            let mut final_download_links = Vec::new();
            for download_link in download_links {
                if !final_download_links.contains(&download_link) {
                    final_download_links.push(download_link);
                }
            }
            let _ = link_information_sender.send(final_download_links);
        });
    });
}

pub fn recognise_links(raw_hosting_service_links: &str) -> Vec<HostingServiceLink> {
    let generic_link_regex = Regex::new(r"(?:https?://)?(?:www\.)?([-a-zA-Z0-9@%._+~#=]{2,256}\.[a-z]{2,6})/([-a-zA-Z0-9@:%_+.~#?&/=]*)").unwrap();
    let mut detected_hoster_links = Vec::new();
    for (link, [domain, slug]) in generic_link_regex
        .captures_iter(raw_hosting_service_links)
        .map(|c| c.extract())
    {
        match domain {
            "multiup.io" | "multiup.org" => {
                if let Ok(multiup_link) = new_multiup_link(link.to_owned(), slug.to_owned()) {
                    if !detected_hoster_links.contains(&multiup_link) {
                        detected_hoster_links.push(multiup_link);
                    }
                };
            }
            "mirrored.to" | "micr.cr" => {
                if let Ok(mirrored_link) = new_mirrored_link(link.to_owned(), slug.to_owned()) {
                    if !detected_hoster_links.contains(&mirrored_link) {
                        detected_hoster_links.push(mirrored_link);
                    }
                };
            }
            _ => {}
        }
    }

    return detected_hoster_links;
}

async fn extract_download_links(
    hosting_service_links: &Vec<HostingServiceLink>,
    recheck_validity: bool,
) -> Vec<LinkInformation> {
    if hosting_service_links.is_empty() {
        return Vec::new();
    }
    let browser = new_browser();
    let max_concurrent_tabs = 10;
    let semaphore = Arc::new(Semaphore::new(max_concurrent_tabs));
    let mut tasks = Vec::new();
    // let client = Client::new();
    // TODO: Properly return errors
    for link in hosting_service_links {
        match link {
            HostingServiceLink::MultiUpProject(project) => {
                let mirror_links = match project.get_mirror_links(&browser) {
                    Ok(mirror_links) => mirror_links,
                    Err(error) => {
                        let original_link = project.original_link.clone();
                        let task = tokio::spawn(async move {
                            let link_information = LinkInformation::new(
                                original_link,
                                String::new(),
                                String::new(),
                                String::new(),
                                0.0,
                                String::new(),
                                Vec::new(),
                                Err(error),
                            );
                            link_information
                        });
                        tasks.push(task);
                        continue;
                    }
                };
                for mirror_link in mirror_links {
                    let browser = browser.clone();
                    let semaphore = Arc::clone(&semaphore);
                    let task = tokio::spawn(async move {
                        let _permit = semaphore.acquire().await.unwrap();
                        mirror_link.get_link_information(&browser, recheck_validity)
                    });
                    tasks.push(task);
                }
            }
            HostingServiceLink::MultiUpMirror(link) => {
                let browser = browser.clone();
                let semaphore = Arc::clone(&semaphore);
                let link = link.to_owned();
                let task = tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    link.get_link_information(&browser, recheck_validity)
                });
                tasks.push(task);
            }
            HostingServiceLink::Mirrored(link) => {
                let browser = browser.clone();
                let semaphore = Arc::clone(&semaphore);
                let link = link.to_owned();
                let task = tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    link.get_link_information(&browser, recheck_validity)
                });
                tasks.push(task);
            }
        }
    }

    let mut link_informations: Vec<LinkInformation> = Vec::new();
    let results = futures::future::join_all(tasks).await;
    for link in results {
        match link {
            Ok(link) => {
                link_informations.push(link);
            }
            Err(error) => {
                eprintln!("Error: {}", error);
            }
        }
    }

    let mut final_link_informations = BTreeMap::new();
    for link_information in link_informations {
        final_link_informations.insert(link_information.id.clone(), link_information);
    }

    let link_informations = final_link_informations.into_values().collect();
    link_informations
}
