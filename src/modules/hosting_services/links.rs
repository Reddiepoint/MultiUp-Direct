use crate::modules::hosting_services::multiup::links::MultiUpMirrorLink;

/// Represents all the different types of hosting services.
pub enum HostingServiceLink {
    MultiUp(MultiUpMirrorLink),
    
}


/// Represents the consolidated main download link.
pub struct DownloadLink {
    original_link: String,
    size: f32,
    size_unit: String,
    direct_links: Vec<HostLink>,
}
pub struct HostLink {
    
}