use headless_chrome::{Browser, Tab};
use rayon::prelude::*;
use regex::Regex;
use scraper::{Html, Selector};
use std::sync::Arc;
use std::time::Duration;
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

        // Click view file links button
        if !self.original_link.contains("?hash=") {
            Self::navigate_to_file_links(&tab);
        }

        Self::click_check_link_status(&tab);

        let page_html = match tab.get_content() {
            Ok(html) => html,
            Err(error) => {
                eprintln!("Error getting content: {}", error);
                panic!(); // TODO: Return error
            }
        };
        let _ = tab.close(true).unwrap();
        let document = Html::parse_document(&page_html);
        let direct_links = Self::grab_direct_links(&document, browser);
        let (filename, size, size_unit) = Self::get_metadata(&document);

        // return LinkInformation {
        //     original_link: self.original_link.clone(),
        //     domain: self.get_domain().to_string(),
        //     filename,
        //     id: self.id.to_string(),
        //     size,
        //     size_unit,
        //     direct_links,
        //     status: Ok(()),
        // };
        LinkInformation::new(
            self.original_link.clone(),
            self.get_domain().to_string(),
            self.id.to_string(),
            filename,
            size,
            size_unit,
            direct_links,
            Ok(()),
        )
    }
}

impl MirroredLink {
    fn navigate_to_file_links(tab: &Arc<Tab>) {
        // TODO: Figure out how to stop ads from interfering
        // let element = wait_for_element(&tab, "button.secondary.larger", None, None).unwrap();
        // element.click();
        // if let Ok(element) = wait_for_element(&tab, "ins", Some(5), None) {
        //     // let element = wait_for_element(&tab, "button.secondary.larger", None, None).unwrap();
        //     if let Ok(element) = tab.find_element("button.secondary.larger") {
        //         element.click();
        //     }
        // }
        //
        // wait_for_element(&tab, "#result", None, None);
        let element = wait_for_element(&tab, "a[href*='mirrored.to/files']", None, None).unwrap();
        let link = element.get_attribute_value("href").unwrap().unwrap();
        tab.navigate_to(&link);
    }
    fn click_check_link_status(tab: &Arc<Tab>) {
        wait_for_element(
            &tab,
            "#result > div.col-sm > table > tbody > tr:nth-child(1) > td:nth-child(1)",
            None,
            None,
        );

        let check_divs = tab
            .find_elements("td[data-label='Link Status'] > span > div:not(#linkstatus1000)")
            .unwrap();

        let check_button_code: Vec<String> = check_divs
            .par_iter()
            .map(|div| div.find_element("a").unwrap().get_attribute_value("onclick").unwrap().unwrap())
            .collect();
        let checks = check_button_code.join(";");
        let check = tab.evaluate(&checks, false);

        // validity_divs.par_iter().for_each(|div| {
        //     let button = div.find_element("a").unwrap();
        //     println!("Clicking button");
        //     button.click();
        // });

        // for div in &check_divs {
        //     div.wait_for_element("em");
        // }

        check_divs.par_iter().for_each(|div| {
            div.wait_for_element_with_custom_timeout("div", Duration::from_secs(10));
        })
    }
    fn get_getlink_link(get_link: &str) -> String {
        format!("https://www.mirrored.to{}", get_link)
    }

    fn get_metadata(html: &Html) -> (String, f32, String) {
        let selector = Selector::parse("h3").unwrap();
        let filename = html
            .select(&selector)
            .next()
            .unwrap()
            .text()
            .next()
            .unwrap()
            .to_owned();
        let selector = Selector::parse("h6 > span").unwrap();
        let size_full = html
            .select(&selector)
            .next()
            .unwrap()
            .text()
            .next()
            .unwrap()
            .to_owned();
        let size = size_full
            .split_whitespace()
            .next()
            .unwrap()
            .parse::<f32>()
            .unwrap();
        let size_unit = size_full.split_whitespace().last().unwrap().to_uppercase();

        (filename, size, size_unit)
    }

    fn grab_direct_links(html: &Html, browser: &Browser) -> Vec<DirectLink> {
        let selector = Selector::parse("td > a[target='_blank']").unwrap();
        let host_links: Vec<String> = html
            .select(&selector)
            .map(|element| element.attr("href").unwrap())
            .map(Self::get_getlink_link)
            .collect();
        println!("{:?}", host_links);
        let selector =
            Selector::parse("td[data-label='Link Status'] > span > div > div").unwrap();
        let validities: Vec<LinkValidity> = html
            .select(&selector)
            .map(|element| element.text().next().unwrap())
            .map(|validity| match validity {
                "Online!" => LinkValidity::Valid,
                "Offline!" => LinkValidity::Invalid,
                _ => LinkValidity::Unknown,
            })
            .collect();
        assert_eq!(validities.len(), host_links.len());
        let hosts = host_links.iter().zip(validities);
        let hosts: Vec<(String, LinkValidity)> = hosts
            .map(|(link, validity)| (link.to_owned(), validity))
            .collect();
        let direct_links = hosts.into_par_iter()
            .map(|(link, validity)| {
                let tab = navigate_to_new_tab(browser, &link);
                let _ = wait_for_element(&tab, "code", None, None);
                let document = Html::parse_document(&tab.get_content().unwrap());
                let _ = tab.close(false).unwrap();

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

#[cfg(test)]
mod tests {
    use crate::modules::hosting_services::browser::{
        navigate_to_new_tab, new_browser, wait_for_element,
    };
    use crate::modules::hosting_services::links::HostingService;
    use crate::modules::hosting_services::mirrored::links::{process_slug, MirroredLink};

    #[test]
    fn test_process_slug() {
        let id =
            process_slug("https://www.mirrored.to/files/0KGTBMFK/Abc_news_radio_intro.mid_links");
        assert_eq!(id, "0KGTBMFK");
    }

    #[test]
    fn test_get_getlink_link() {
        let getlink_link = "/getlink/0KGTBMFK/31/?hid=VExta0VRVUtMeEY5WHd0eFlBZG1lTUwzR0xJdWkwVG5kS3pxcWorZXVUST0qOipJX7mX%2BILll7jRRIIRDYa4&tid=91b733d3e3a3797a93bc5095b7dab4c1";
        let link = MirroredLink::get_getlink_link(getlink_link);
        assert_eq!(link, "https://www.mirrored.to/getlink/0KGTBMFK/31/?hid=VExta0VRVUtMeEY5WHd0eFlBZG1lTUwzR0xJdWkwVG5kS3pxcWorZXVUST0qOipJX7mX%2BILll7jRRIIRDYa4&tid=91b733d3e3a3797a93bc5095b7dab4c1")
    }
    #[test]
    #[ignore]
    fn test_navigate_to_view_file_links() {
        let url = "https://www.mirrored.to/files/0KGTBMFK/Abc_news_radio_intro.mid_links";

        for i in 0..1 {
            println!("Iteration {}", i);
            let browser = new_browser();
            let tab = navigate_to_new_tab(&browser, url);
            MirroredLink::navigate_to_file_links(&tab);

            assert!(wait_for_element(&tab, "#result", None, None).is_ok());
        }
    }

    #[test]
    #[ignore]
    fn test_click_check_link_status() {
        let url =
            "https://www.mirrored.to/files/0KGTBMFK/?hash=c90ecbc39202a5057406628097f00c53&dl=0";
        for i in 0..1 {
            println!("Iteration {}", i);
            let browser = new_browser();
            let tab = navigate_to_new_tab(&browser, url);
            MirroredLink::click_check_link_status(&tab);
            let statuses = tab
                .find_elements("td[data-label='Link Status'] > span > div > div")
                .unwrap();
            assert_eq!(statuses.len(), 3);
        }
    }

    #[test]
    #[ignore]
    fn test_get_link_information() {
        for i in 0..1 {
            println!("Iteration {}", i);
            let url = "https://www.mirrored.to/files/0KGTBMFK/Abc_news_radio_intro.mid_links";
            let mirrored_link = MirroredLink {
                original_link: url.to_string(),
                id: "0KGTBMFK".to_string(),
            };

            let browser = new_browser();
            // let tab = navigate_to_new_tab(&browser, url);
            let link_information = mirrored_link.get_link_information(&browser, false);
            println!("{:?}", link_information);
            assert_eq!(link_information.direct_links.len(), 3);
        }
    }
}
