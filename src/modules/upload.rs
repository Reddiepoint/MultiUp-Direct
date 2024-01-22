use reqwest::{Client, multipart};
use std::error::Error;
use eframe::egui::{Context, TextBuffer, Ui};
use crate::modules::api::{get_fastest_server, MultiUpUploadResponse};

#[derive(Default)]
pub struct UploadUI {
    
}

impl UploadUI {
    pub fn display(ctx: &Context, ui: &mut Ui, upload_ui: &mut UploadUI) {
        
    }
}

#[tokio::test]
async fn test_download_and_upload_file_with_reqwest() {
    let url = vec!["https://v2w3x4.debrid.it/dl/2zmoaog89f0/cis-33828253.pdf", "https://v2w3x4.debrid.it/dl/2zmtn7j4919/cis-33828253.pdf"];
    match download_and_upload_file_with_reqwest(&url).await {
        Ok(_response) => {
            // println!("{}", response.url.unwrap());
        }
        Err(error) => {
            eprintln!("{}", error);
        }
    };
}

async fn download_and_upload_file_with_reqwest(download_urls: &[&str]) -> Result<(), Box<dyn Error>> {
    let api_url = get_fastest_server().await?;

    // Create a reqwest client
    let client = Client::new();

    // Download the file
    let mut responses = vec![];
    for download_url in download_urls {
        let download_response = client.get(download_url.as_str()).send().await?;
        responses.push(download_response);
    }

    let mut files = vec![];
    for download_response in responses {
        let content_disposition = download_response.headers().get(reqwest::header::CONTENT_DISPOSITION);
        let file_name = content_disposition
            .and_then(|cd| cd.to_str().ok())
            .and_then(|cd| cd.split(';').find(|&s| s.trim_start().starts_with("filename=")))
            .and_then(|filename_param| filename_param.split('=').nth(1))
            .map(|name| name.trim_matches('"').to_string());


        let content_length = download_response.headers().get(reqwest::header::CONTENT_LENGTH)
            .and_then(|cl| cl.to_str().ok())
            .and_then(|cl| cl.parse::<u64>().ok());

        // Stream the file directly without saving to disk, converting it to a compatible stream
        let file_stream = download_response.bytes_stream();

        // Convert the stream into a Body for the multipart form
        let file_body = reqwest::Body::wrap_stream(file_stream);

        // Create a multipart/form-data object with the stream
        let part = multipart::Part::stream_with_length(file_body, content_length.unwrap_or(0))
            .file_name(file_name.unwrap_or("file_name".to_string()));

        files.push(part);
    }

    // Create a multipart/form-data object
    let mut form = multipart::Form::new()
        // .part("files", part)
        .text("project-hash", "testing");

    for part in files {
        form = form.part("files[]", part);
    }

    // Upload the file
    let response = client.post(api_url)
        .multipart(form)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await?;
        eprintln!("Error uploading file: {}", error_text);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Upload failed")));
    }

    // Output the response body for the upload
    let upload_response = response.json::<MultiUpUploadResponse>().await?;
    match upload_response.files.is_empty() {
        true => {
            eprintln!("No files in the upload response");
        }
        false => {
            println!("Upload Response: {:?}", upload_response);
        }
    }

    Ok(())
}
