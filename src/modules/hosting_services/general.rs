use regex::Regex;

pub fn extract_raw_links(raw_hosting_service_links: String) {

}


fn recognise_links(raw_hosting_service_links: String) {
    let generic_link_regex = Regex::new(r"(?:https?://)?(?:www\.)?([-a-zA-Z0-9@%._+~#=]{2,256}\.[a-z]{2,6})/([-a-zA-Z0-9@:%_+.~#?&/=]*)").unwrap();
    let mut detected_hoster_links = vec![];
    for (link, [domain, slug]) in generic_link_regex.captures_iter(raw_hosting_service_links.as_str()).map(|c| c.extract()) {
        match domain {
            "multiup.io" | "multiup.org" => todo!(),
            "mirrored.to" | "micr.cr" => todo!(),
            _ => {},
        }
        detected_hoster_links.push(link.to_owned());
    }
    
    
}