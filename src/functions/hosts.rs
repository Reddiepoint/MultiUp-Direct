use async_recursion::async_recursion;
use crate::structs::hosts::{LinkInformation, Url};

#[async_recursion]
pub async fn check_validity(url: &str) -> LinkInformation {
    let client = reqwest::Client::new();
    match client.post("https://multiup.org/api/check-file")
        .form(&Url { link: url.to_string()})
        .send().await.unwrap().json::<LinkInformation>().await {
        Ok(information) => information,
        Err(error) => {
            check_validity(url).await
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


