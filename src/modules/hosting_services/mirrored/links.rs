use headless_chrome::Browser;
use rayon::prelude::*;
use regex::Regex;
use scraper::{Html, Selector};

use crate::modules::hosting_services::browser::{navigate_to_new_tab, wait_for_element};
use crate::modules::hosting_services::links::{
    DirectLink, HostingService, HostingServiceLink, LinkInformation, LinkValidity,
};

#[derive(Clone)]
pub struct MirroredLink {
    original_link: String,
    id: String,
}

impl HostingService for MirroredLink {
    fn get_id(&self) -> &str {
        &self.id
    }

    fn get_domain(&self) -> &str {
        "mirrored.to"
    }

    fn get_link_information(&self, browser: &Browser, _recheck_validity: bool) -> LinkInformation {
        let tab = navigate_to_new_tab(browser, &self.original_link);
        if !self.original_link.contains("?hash=") {
            let element = wait_for_element(&tab, "button.secondary.larger", None, None).unwrap();

            element.click();
            wait_for_element(&tab, "caption", None, None);
        }

        let validity_divs = tab.find_elements("td[data-label='Link Status'] > span > div:not(#linkstatus1000)").unwrap();

        for div in &validity_divs {
            let button = div.find_element("a").unwrap();
            button.click();
        }

        for div in &validity_divs {
            div.wait_for_element("em");
        }

        let page_html = match tab.get_content() {
            Ok(html) => html,
            Err(error) => {
                eprintln!("Error getting content: {}", error);
                panic!(); // TODO: Return error
            }
        };
        tab.close(false);
        let document = Html::parse_document(&page_html);
        let direct_links = Self::grab_direct_links(&document, browser);
        let (filename, size, size_unit) = Self::get_metadata(&document);

        return LinkInformation {
            original_link: self.original_link.clone(),
            domain: self.get_domain().to_string(),
            filename,
            size,
            size_unit,
            direct_links,
            status: Ok(()),
        };
    }
}

impl MirroredLink {
    fn get_getlink_link(get_link: &str) -> String {
        format!("https://mirrored.to/{}", get_link)
    }

    fn get_metadata(html: &Html) -> (String, f32, String) {
        let selector = Selector::parse("h3").unwrap();
        let filename = html.select(&selector).next().unwrap().text().next().unwrap().to_owned();
        let selector = Selector::parse("h6 > span").unwrap();
        let size_full = html.select(&selector).next().unwrap().text().next().unwrap().to_owned();
        let size = size_full.split_whitespace().next().unwrap().parse::<f32>().unwrap();
        let size_unit = size_full.split_whitespace().last().unwrap().to_uppercase();

        return (filename, size, size_unit);
    }
    fn grab_direct_links(html: &Html, browser: &Browser) -> Vec<DirectLink> {
        let selector = Selector::parse("td > a[target='_blank']").unwrap();
        let host_links: Vec<String> = html.select(&selector).map(|element| element.attr("href").unwrap()).map(Self::get_getlink_link).collect();
        println!("{:?}", host_links);
        let selector = Selector::parse("td[data-label='Link Status'] > span > div > div > em > b").unwrap();
        let validities: Vec<LinkValidity> = html.select(&selector).map(|element| element.text().next().unwrap()).map(|validity| match validity {
            "Online!" => LinkValidity::Valid,
            "Offline" => LinkValidity::Invalid,
            _ => LinkValidity::Unknown,
        }).collect();
        let hosts = host_links.iter().zip(validities);
        let hosts: Vec<(String, LinkValidity)> = hosts.map(|(link, validity)| (link.to_owned(), validity)).collect();
        let direct_links = hosts.into_par_iter()
            .map(|(link, validity)| {
                let tab = navigate_to_new_tab(browser, &link);
                let _ = wait_for_element(&tab, "code", None, None);
                let document = Html::parse_document(&tab.get_content().unwrap());
                tab.close(false);

                let selector = Selector::parse("code").unwrap();
                let link = document.select(&selector).next().unwrap().text().next().unwrap().to_owned();

                let generic_link_regex = Regex::new(r"(?:https?://)?(?:www\.)?([-a-zA-Z0-9@%._+~#=]{2,256}\.[a-z]{2,6})/([-a-zA-Z0-9@:%_+.~#?&/=]*)").unwrap();
                let (_link, [domain, _slug]) = generic_link_regex.captures(&link).unwrap().extract();
                DirectLink::new(domain.to_string(), link, validity)
            })
            .collect();

        return direct_links;
    }
}

pub fn new_mirrored_link(original_link: String, slug: String) -> HostingServiceLink {
    let id = process_slug(&slug);
    HostingServiceLink::Mirrored(MirroredLink { original_link, id })
}

fn process_slug(slug: &str) -> String {
    for path in slug.split('/') {
        if path.len() == 8 {
            return path.to_owned();
        }
    }
    panic!()
}
