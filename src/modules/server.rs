use serde::Deserialize;

#[derive(Default, Deserialize)]
pub struct FastestServer {
    pub error: String,
    pub server: String,
}