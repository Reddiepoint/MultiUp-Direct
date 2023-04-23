use crate::structs::login::{
    LoginData,
};

use std::error::Error;

use std::future::Future;
use std::io;
use std::io::Read;
use std::path::Path;
use reqwest::{Body, multipart};
use reqwest::multipart::Part;
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};
use crate::structs::server::FastestServer;
use crate::structs::upload::UploadResponse;


pub async fn get_upload_url() -> Result<FastestServer, reqwest::Error> {
    let server = reqwest::get("https://multiup.org/api/get-fastest-server").await?.json::<FastestServer>().await;
    server

}

pub async fn upload(upload_url: FastestServer) -> String {
    let file_path = r#"C:\Users\matth\Downloads\test.txt"#;
    let file_name = "test.txt";

    let file_content = tokio::fs::read(file_path).await.unwrap();
    let part = multipart::Part::bytes(file_content).file_name(file_name);

    let form = multipart::Form::new().part("files", part);

    let client = reqwest::Client::new();
    let response = client.post(upload_url.server).multipart(form).send().await.unwrap().json::<UploadResponse>().await.unwrap();

    println!("{:?}", response.files[0].url);

    String::new()



}