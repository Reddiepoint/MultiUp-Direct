
pub struct DebridServiceLinks {
    pub link: String,
    pub status: Result<(), DebridAPIError>,
}

pub enum DebridAPIError {
    Unknown
}