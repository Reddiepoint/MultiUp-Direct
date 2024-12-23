use regex::Regex;

pub struct DebridServiceLinks {
    pub link: String,
    pub status: Result<(), DebridAPIError>,
}

pub enum DebridAPIError {
    Unknown
}

pub fn recognise_links(raw_links: &str) -> Vec<String> {
    let generic_link_regex = Regex::new(r"(?:https?://)?(?:www\.)?([-a-zA-Z0-9@%._+~#=]{2,256}\.[a-z]{2,6})/([-a-zA-Z0-9@:%_+.~#?&/=]*)").unwrap();
    let mut detected_links = Vec::new();
    for (link, [domain, slug]) in generic_link_regex
        .captures_iter(raw_links)
        .map(|c| c.extract()) {
        detected_links.push(link.to_owned());
    }
    detected_links
}

