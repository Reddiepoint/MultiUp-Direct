use std::time::Duration;
use async_recursion::async_recursion;
use crossbeam_channel::{Receiver, TryRecvError};
use reqwest::{Client, StatusCode};
use crate::modules::links::LinkError;

#[async_recursion]
pub async fn get_page_html(page_link: &str, client: &Client, cancel_receiver: Option<Receiver<bool>>, try_count: u8) -> Result<String, LinkError> {
    if let Some(receiver) = cancel_receiver.clone() {
        if let Ok(_) | Err(TryRecvError::Disconnected) = receiver.try_recv() {
            return Err(LinkError::Cancelled);
        }
    }

    if try_count >= 10 {
        return Err(LinkError::TimedOut);
    }

    let server_response = match client.get(page_link).send().await {
        Ok(response) => response,
        Err(error) => return Err(LinkError::Reqwest(error))
    };

    return match server_response.error_for_status() {
        Ok(res) => Ok(res.text().await.unwrap().to_string()),
        Err(error) => {
            // Repeat if error is not 404
            if error.status().unwrap() != StatusCode::NOT_FOUND {
                let _ = tokio::time::sleep(Duration::from_millis(100)).await;
                return get_page_html(page_link, client, cancel_receiver, try_count + 1).await;
            }
            Err(LinkError::Invalid)
        }
    };
}