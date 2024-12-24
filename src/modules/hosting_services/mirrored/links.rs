use crate::modules::hosting_services::browser::{navigate_to_new_tab, wait_for_element};
use crate::modules::hosting_services::links::{
    DirectLink, DirectLinkValidity, HostingService, HostingServiceLink, LinkError, LinkInformation,
};
use headless_chrome::{Browser, Tab};
use rayon::prelude::*;
use regex::Regex;
use scraper::{Html, Selector};
use std::sync::Arc;
use std::time::Duration;

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
        let mut link_information = LinkInformation::new(
            self.original_link.clone(),
            self.get_domain().to_string(),
            self.get_id().to_string(),
            String::new(),
            0.0,
            String::new(),
            Vec::new(),
            Ok(()),
        );
        let tab = match navigate_to_new_tab(browser, &self.original_link) {
            Ok(tab) => tab,
            Err((tab, error)) => {
                link_information.status = Err(error);
                if let Some(tab) = tab {
                    if let Err(error) = tab.close(true) {
                        link_information.status = Err(LinkError::BrowserError(format!(
                            "Failed to close tab: {}",
                            error
                        )));
                    }
                }
                return link_information;
            }
        };

        if let Ok(_warning) = tab.find_element(".error") {
            link_information.status = Err(LinkError::InvalidLink("File not found".to_string()));
            if let Err(error) = tab.close(true) {
                link_information.status = Err(LinkError::BrowserError(format!(
                    "Failed to close tab: {}",
                    error
                )));
            };
            return link_information;
        }
        // Click view file links button
        if !self.original_link.contains("?hash=") {
            if let Err(error) = Self::navigate_to_file_links(&tab) {
                link_information.status = Err(error);
                return link_information;
            };
        }

        if let Err(error) = Self::click_check_link_status(&tab) {
            link_information.status = Err(error);
            return link_information;
        };

        let page_html = match tab.get_content() {
            Ok(html) => html,
            Err(error) => {
                eprintln!("Error getting content: {}", error);
                link_information.status =
                    Err(LinkError::BrowserError("Failed to get HTML".to_string()));
                return link_information;
            }
        };

        if let Err(error) = tab.close(true) {
            link_information.status = Err(LinkError::BrowserError(format!(
                "Failed to close tab: {}",
                error
            )));
            return link_information;
        };

        let document = Html::parse_document(&page_html);
        let direct_links = match Self::grab_direct_links(&document, browser) {
            Ok(direct_links) => direct_links,
            Err(error) => {
                link_information.status = Err(error);
                return link_information;
            }
        };
        let (filename, size, size_unit) = match Self::get_metadata(&document) {
            Ok((filename, size, size_unit)) => (filename, size, size_unit),
            Err(error) => {
                link_information.status = Err(error);
                return link_information;
            }
        };

        link_information.filename = filename;
        link_information.size = size;
        link_information.size_unit = size_unit;
        link_information.direct_links = direct_links;
        link_information
    }
}

impl MirroredLink {
    fn navigate_to_file_links(tab: &Arc<Tab>) -> Result<(), LinkError> {
        let element = wait_for_element(tab, "a[href*='mirrored.to/files']", None, None)?;
        let link = element
            .get_attribute_value("href")
            .map_err(|_error| {
                LinkError::BrowserError("Failed to get link for direct links page".to_string())
            })?
            .ok_or_else(|| {
                LinkError::BrowserError("Failed to get link for direct links page".to_string())
            })?;
        if let Err(error) = tab.navigate_to(&link) {
            return Err(LinkError::BrowserError(format!(
                "Failed to navigate to file links: {}",
                error
            )));
        };
        Ok(())
    }

    fn click_check_link_status(tab: &Arc<Tab>) -> Result<(), LinkError> {
        wait_for_element(
            tab,
            "#result > div.col-sm > table > tbody > tr:nth-child(1) > td:nth-child(1)",
            None,
            None,
        )?;

        let check_divs = tab
            .find_elements("td[data-label='Link Status'] > span > div:not(#linkstatus1000)")
            .map_err(|error| {
                LinkError::BrowserError(format!("Failed to find check divs: {}", error))
            })?;

        let check_button_code: Vec<String> = check_divs
            .par_iter()
            .map(|div| {
                div.find_element("a")
                    .and_then(|a| a.get_attribute_value("onclick"))
            })
            .map(|result| result.unwrap_or(None).unwrap_or("Failed".to_string()))
            .collect();
        if check_button_code.contains(&"Failed".to_string()) {
            return Err(LinkError::BrowserError(
                "Failed to get links from buttons".to_string(),
            ));
        }
        let checks = check_button_code.join(";");
        if let Err(error) = tab.evaluate(&checks, false) {
            return Err(LinkError::BrowserError(format!(
                "Failed to open new tabs: {}",
                error
            )));
        };

        // validity_divs.par_iter().for_each(|div| {
        //     let button = div.find_element("a").unwrap();
        //     println!("Clicking button");
        //     button.click();
        // });

        // for div in &check_divs {
        //     div.wait_for_element("em");
        // }

        let link_status: Vec<&str> = check_divs
            .par_iter()
            .map(|div| {
                if let Err(_error) =
                    div.wait_for_element_with_custom_timeout("div", Duration::from_secs(10))
                {
                    return "Failed";
                };
                "Success"
            })
            .collect();

        if link_status.contains(&"Failed") {
            return Err(LinkError::BrowserError(
                "Link status failed to load".to_string(),
            ));
        }

        Ok(())
    }
    fn get_getlink_link(get_link: &str) -> String {
        format!("https://www.mirrored.to{}", get_link)
    }

    fn get_metadata(html: &Html) -> Result<(String, f32, String), LinkError> {
        let selector = Selector::parse("h3").map_err(|error| {
            LinkError::BrowserError(format!("Failed to parse selector `h3`: {}", error))
        })?;
        let filename = html
            .select(&selector)
            .next()
            .and_then(|element| element.text().next())
            .ok_or_else(|| LinkError::BrowserError("Failed to get filename".to_string()))?;

        let selector = Selector::parse("h6 > span").map_err(|error| {
            LinkError::BrowserError(format!("Failed to parse selector `h6 > span`: {}", error))
        })?;
        let size_full = html
            .select(&selector)
            .next()
            .and_then(|element| element.text().next())
            .ok_or_else(|| LinkError::BrowserError("Failed to get file size".to_string()))?;
        let size = size_full
            .split_whitespace()
            .next()
            .and_then(|size| size.parse::<f32>().ok())
            .ok_or_else(|| LinkError::BrowserError("Failed to parse file size".to_string()))?;
        let size_unit = size_full
            .split_whitespace()
            .last()
            .ok_or_else(|| LinkError::BrowserError("Failed to parse file size unit".to_string()))?
            .to_uppercase();

        Ok((filename.to_owned(), size.to_owned(), size_unit))
    }

    fn grab_direct_links(html: &Html, browser: &Browser) -> Result<Vec<DirectLink>, LinkError> {
        let selector = Selector::parse("td > a[target='_blank']").map_err(|error| {
            LinkError::BrowserError(format!(
                "Failed to parse selector `td > a[target='_blank']`: {}",
                error
            ))
        })?;

        let host_links: Vec<Option<String>> = html
            .select(&selector)
            .map(|element| element.attr("href").map(Self::get_getlink_link))
            .collect();
        if host_links.iter().any(|links| links.is_none()) {
            return Err(LinkError::BrowserError(
                "Failed to get host links".to_string(),
            ));
        }

        let selector = Selector::parse("td[data-label='Link Status'] > span > div > div").map_err(
            |error| {
                LinkError::BrowserError(format!(
                    "Failed to parse selector `td[data-label='Link Status'] > span > div > div`: {}",
                    error
                ))
            },
        )?;
        // let validities: Vec<Option<&str>>= html.select(&selector).map(|element| element.text().next()).collect();
        // if validities.iter().any(|validity| validity.is_none()) {
        //     return Err(LinkError::BrowserError("Failed to get status text".to_string()));
        // }
        // let validities: Vec<DirectLinkValidity> = validities.iter()
        //     .map(|validity| match validity.unwrap_or_default() {
        //         "Online!" => DirectLinkValidity::Valid,
        //         "Offline!" => DirectLinkValidity::Invalid,
        //         _ => DirectLinkValidity::Unknown,
        //     })
        //     .collect();

        let validities: Vec<Option<DirectLinkValidity>> = html
            .select(&selector)
            .map(|element| {
                element.text().next().map(|text| match text {
                    "Online!" => DirectLinkValidity::Valid,
                    "Offline!" => DirectLinkValidity::Invalid,
                    _ => DirectLinkValidity::Unknown,
                })
            })
            .collect();
        if validities.iter().any(|validity| validity.is_none()) {
            return Err(LinkError::BrowserError(
                "Failed to get status text".to_string(),
            ));
        }
        assert_eq!(validities.len(), host_links.len());

        let hosts = host_links.iter().zip(validities);
        let hosts: Vec<(String, DirectLinkValidity)> = hosts
            .map(|(link, validity)| (link.clone().unwrap(), validity.unwrap()))
            .collect();
        let direct_links: Vec<Result<DirectLink, LinkError>> = hosts.into_par_iter()
            .map(|(link, validity)| {
                let tab = match navigate_to_new_tab(browser, &link) {
                    Ok(tab) => tab,
                    Err((tab, error)) => {
                        if let Some(tab) = tab {
                            if let Err(error) = tab.close(true) {
                                return Err(LinkError::BrowserError(format!(
                                    "Failed to close tab: {}",
                                    error
                                )));
                            }
                        }
                        return Err(error);
                    }
                };
                wait_for_element(&tab, "code", None, None)?;
                let page_html = match tab.get_content() {
                    Ok(html) => html,
                    Err(error) => {
                        eprintln!("Error getting content: {}", error);
                        return Err(LinkError::BrowserError("Failed to get HTML".to_string()));
                    }
                };
                let document = Html::parse_document(&page_html);
                if let Err(error) = tab.close(true) {
                    return Err(LinkError::BrowserError(format!(
                        "Failed to close tab: {}",
                        error
                    )));
                };

                let selector = Selector::parse("code").map_err(|error| {
                    LinkError::BrowserError(format!(
                        "Failed to parse selector `code`: {}",
                        error
                    ))
                })?;

                let link = document.select(&selector).next().and_then(|element| element.text().next()).map(|text| text.to_owned());
                let link = match link {
                    Some(link) => link,
                    None => {
                        return Err(LinkError::BrowserError(
                            "Failed to get direct link".to_string(),
                        ));
                    }
                };

                let generic_link_regex = Regex::new(r"(?:https?://)?(?:www\.)?([-a-zA-Z0-9@%._+~#=]{2,256}\.[a-z]{2,6})/([-a-zA-Z0-9@:%_+.~#?&/=]*)").unwrap();
                let (_link, [domain, _slug]) = generic_link_regex.captures(&link).unwrap().extract();
                Ok(DirectLink::new(domain.to_string(), link, validity))
            })
            .collect();
        let mut final_links: Vec<DirectLink> = Vec::new();
        for direct_link in direct_links.into_iter() {
            let link = direct_link?;
            final_links.push(link)
        }
        Ok(final_links)
    }
}

pub fn new_mirrored_link(
    original_link: String,
    slug: String,
) -> Result<HostingServiceLink, LinkError> {
    let id = process_slug(&slug)?;
    Ok(HostingServiceLink::Mirrored(MirroredLink {
        original_link,
        id,
    }))
}

fn process_slug(slug: &str) -> Result<String, LinkError> {
    for path in slug.split('/') {
        if path.len() == 8 {
            return Ok(path.to_owned());
        }
    }
    Err(LinkError::InvalidLink("Invalid slug".to_string()))
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
            process_slug("https://www.mirrored.to/files/0KGTBMFK/Abc_news_radio_intro.mid_links")
                .expect("Expected valid slug");
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
            let tab = match navigate_to_new_tab(&browser, url) {
                Ok(tab) => tab,
                Err(_) => panic!()
            };
            MirroredLink::navigate_to_file_links(&tab);

            assert!(wait_for_element(&tab, "#result", None, None).is_ok());
        }
    }

    #[test]
    #[ignore]
    fn test_click_check_link_status() {
        // Note: Hash changes over time
        let url =
            "https://www.mirrored.to/files/0KGTBMFK/?hash=b8ccdcb3cad258b2bb8dc7dc80a29477&dl=0";
        for i in 0..1 {
            println!("Iteration {}", i);
            let browser = new_browser();
            let tab = match navigate_to_new_tab(&browser, url) {
                Ok(tab) => tab,
                Err(_) => panic!("Failed to navigate to tab")
            };
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
