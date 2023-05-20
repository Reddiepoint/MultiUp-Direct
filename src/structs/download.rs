use crate::structs::hosts::{MirrorLink};


pub struct Download {
    multiup_links: String,
    mirror_links: Vec<MirrorLink>,
    recheck_status: bool,
    display_links: Vec<String>
}