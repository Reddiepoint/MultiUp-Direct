use std::fs::File;
use eframe::egui::Context;
use serde::Deserialize;

#[derive(Default)]
pub struct Upload {
    pub files: Vec<File>,
}

impl Upload {
    pub fn show(ctx: &Context) {

    }
}

#[derive(Debug, Deserialize)]
pub struct FileData {
    name: String,
    size: u64,
    #[serde(rename = "type")]
    file_type: String,
    hash: String,
    user: String,
    md5: String,
    sha: String,
    project: String,
    pub url: String,
    deleteUrl: String,
    deleteType: String
}

#[derive(Deserialize)]
pub struct UploadResponse {
    pub files: Vec<FileData>
}

