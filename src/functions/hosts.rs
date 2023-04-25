use std::collections::HashMap;
use std::sync::mpsc;
use tokio::runtime::Runtime;
use crate::functions::login::login;
use crate::structs::hosts::{AvailableHostsResponse, LinkValidityResponse, Url};

pub async fn check_validity(url: &str) -> HashMap<String, Option<String>>{
    let client = reqwest::Client::new();
    let response = client.post("https://multiup.org/api/check-file")
        .form(&Url { link: url.to_string()})
        .send().await.unwrap().json::<LinkValidityResponse>().await.unwrap();
    response.hosts
}

pub fn get_available_hosts() -> Vec<String> {
    let rt = Runtime::new().expect("Unable to create runtime");
    let _ = rt.enter();
    let (tx, rx) = mpsc::sync_channel(0);
    std::thread::spawn(move || {
        let host_list = rt.block_on(async {
            let response = reqwest::get("https://multiup.org/api/get-list-hosts").await.unwrap().json::<AvailableHostsResponse>().await.unwrap();
            let mut list = vec![];
            for (i, _j) in response.hosts {
                list.push(i);
            };
            list
        });
        tx.send(host_list)
    });

    rx.recv().unwrap()
}


