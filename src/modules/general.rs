use std::time::Duration;
use async_recursion::async_recursion;
use crossbeam_channel::{Receiver, TryRecvError};
use reqwest::{Client, StatusCode};
use crate::modules::links::LinkError;

#[async_recursion]
pub async fn get_page_html(page_link: &str, client: &Client, cancel_signal_receiver: Receiver<bool>) -> Result<String, LinkError> {
    match cancel_signal_receiver.try_recv() {
        Ok(_) | Err(TryRecvError::Disconnected) => {
            return Err(LinkError::Cancelled);
        }
        Err(TryRecvError::Empty) => {}
    };

    let server_response = match client.get(page_link).send().await {
        Ok(response) => response,
        Err(error) => return Err(LinkError::Reqwest(error))
    };

    match server_response.error_for_status() {
        Ok(res) => Ok(res.text().await.unwrap().to_string()),
        Err(error) => {
            // Repeat if error is not 404
            if error.status().unwrap() != StatusCode::NOT_FOUND {
                let _ = tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(LinkError::Invalid)
        }
    }
}