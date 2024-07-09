use crate::modules::hosting_services::browser::new_browser;
use crate::modules::hosting_services::links::{
    DirectLink, HostingService, HostingServiceLink, LinkInformation,
};
use crate::modules::hosting_services::multiup::links::{new_multiup_link, MultiUpMirrorLink};
use headless_chrome::Browser;
use regex::Regex;
use reqwest::Client;
use std::sync::Arc;
use std::thread;
use tokio::runtime::Runtime;
use tokio::sync::Semaphore;

pub fn extract_raw_links(
    raw_hosting_service_links: String,
    recheck_validity: bool,
) -> Vec<LinkInformation> {
    let hosting_service_links = recognise_links(&raw_hosting_service_links);
    let rt = Runtime::new().unwrap();
    thread::spawn(move || {
        rt.block_on(async {
            let download_links = extract_download_links(
                &hosting_service_links,
                recheck_validity, /*, cancel_receiver*/
            )
            .await;
            println!("{:?}", download_links);
            // let _ = direct_links_sender.send(download_links);
        });
    });

    return vec![];
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
                let multiup_link = new_multiup_link(link.to_owned(), slug.to_owned());
                if !detected_hoster_links.contains(&multiup_link) {
                    detected_hoster_links.push(new_multiup_link(link.to_owned(), slug.to_owned()));
                }
            }
            "mirrored.to" | "micr.cr" => todo!(),
            _ => {}
        }
    }

    return detected_hoster_links;
}

async fn extract_download_links(
    hosting_service_links: &Vec<HostingServiceLink>,
    recheck_validity: bool,
) -> Vec<LinkInformation> {
    let browser = new_browser();
    let semaphore = Arc::new(Semaphore::new(100));
    let mut tasks = Vec::new();
    let client = Client::new();
    for link in hosting_service_links {
        match link {
            HostingServiceLink::MultiUpProject(project) => {
                let mirror_links = project.get_mirror_links(&browser).await;
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
            HostingServiceLink::MultiUpMirror(link) => {}
        }
    }
    
    let mut link_informations = Vec::new();
    let results = futures::future::join_all(tasks).await;
    for link in results {
        link_informations.push(link.unwrap());
    }
    return link_informations;
}
