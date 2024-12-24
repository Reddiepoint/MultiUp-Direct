use crate::modules::hosting_services::browser::{navigate_to_new_tab, wait_for_element};
use crate::modules::hosting_services::extract::recognise_links;
use crate::modules::hosting_services::links::{
    DirectLink, DirectLinkValidity, HostingService, HostingServiceLink, LinkError, LinkInformation,
};
use headless_chrome::Browser;
use scraper::node::Element;
use scraper::selectable::Selectable;
use scraper::{Html, Selector};

pub struct MultiUpProjectLink {
    pub original_link: String,
    id: String,
}

impl HostingService for MultiUpProjectLink {
    fn get_id(&self) -> &str {
        &self.id
    }

    fn get_domain(&self) -> &str {
        "multiup.io"
    }

    /// Unimplemented.
    fn get_link_information(&self, _browser: &Browser, _recheck_validity: bool) -> LinkInformation {
        unimplemented!()
    }
}
impl MultiUpProjectLink {
    pub fn new(original_link: String, id: String) -> Self {
        Self { original_link, id }
    }

    fn return_project_link(&self) -> String {
        format!("https://multiup.io/en/project/{}", self.id)
    }

    pub fn get_mirror_links(&self, browser: &Browser) -> Result<Vec<MultiUpMirrorLink>, LinkError> {
        let tab = navigate_to_new_tab(browser, &self.return_project_link());

        // wait_for_element(
        //     &tab,
        //     "h2",
        //     Some(20),
        //     Some(5),
        // )?;

        if let Ok(warning) = tab.find_element(".alert.alert-danger > strong") {
            let error = match warning.get_inner_text() {
                Ok(text) => match text.as_str() {
                    "The requested file could not be found" => {
                        Err(LinkError::InvalidLink("Project not found".to_string()))
                    }
                    _ => Err(LinkError::InvalidLink(text)),
                },
                Err(_error) => Err(LinkError::InvalidLink(
                    "Error getting warning text".to_string(),
                )),
            };

            if let Err(error) = tab.close(true) {
                return Err(LinkError::BrowserError(format!(
                    "Failed to close tab: {}",
                    error
                )));
            };
            return error;
        };

        wait_for_element(&tab, "#textarea-links-long", None, None)?;
        // match tab.wait_for_element("#textarea-links-long")
        // {
        //     Ok(_) => {}
        //     Err(error) => {
        //         eprintln!("Error waiting for element: {}", error);
        //         panic!()
        //     }
        // };

        let page_html = match tab.get_content() {
            Ok(html) => html,
            Err(error) => {
                eprintln!("Error getting content: {}", error);
                return Err(LinkError::BrowserError("Failed to get HTML".to_string()));
            }
        };

        if let Err(error) = tab.close(true) {
            return Err(LinkError::BrowserError(format!(
                "Failed to close tab: {}",
                error
            )));
        };

        let document = Html::parse_document(&page_html);
        let selector = Selector::parse("#textarea-links-long").unwrap();

        let mirror_links = match document.select(&selector).next() {
            Some(element) => match element.text().next() {
                Some(text) => text,
                None => {
                    return Err(LinkError::BrowserError(
                        "Failed to get links from project".to_string(),
                    ));
                }
            },
            None => {
                return Err(LinkError::BrowserError(
                    "Failed to get links from project".to_string(),
                ));
            }
        };
        let links = recognise_links(mirror_links);

        Ok(links
            .into_iter()
            .map(|link| match link {
                HostingServiceLink::MultiUpMirror(link) => link,
                _ => unreachable!(),
            })
            .collect())
    }
}

#[derive(Clone)]
pub struct MultiUpMirrorLink {
    original_link: String,
    id: String,
}

impl HostingService for MultiUpMirrorLink {
    fn get_id(&self) -> &str {
        &self.id
    }

    fn get_domain(&self) -> &str {
        "multiup.io"
    }

    fn get_link_information(&self, browser: &Browser, recheck_validity: bool) -> LinkInformation {
        let mut link_information = LinkInformation::new(
            self.original_link.clone(),
            "multiup.io".to_string(),
            self.id.clone(),
            String::new(),
            0.0,
            String::new(),
            Vec::new(),
            Ok(()),
        );

        let tab = navigate_to_new_tab(browser, &self.return_mirror_link());
        if let Err(error) = wait_for_element(&tab, "h2", Some(20), Some(5)) {
            link_information.status = Err(error);
            return link_information;
        };

        if let Ok(warning) = tab.find_element(".alert.alert-danger > strong") {
            let error = match warning.get_inner_text() {
                Ok(text) => match text.as_str() {
                    "The requested file could not be found" => {
                        Err(LinkError::InvalidLink("File not found".to_string()))
                    }
                    _ => Err(LinkError::InvalidLink(text)),
                },
                Err(_error) => Err(LinkError::InvalidLink(
                    "Error getting warning text".to_string(),
                )),
            };
            link_information.status = error;
            if let Err(error) = tab.close(true) {
                link_information.status = Err(LinkError::BrowserError(format!(
                    "Failed to close tab: {}",
                    error
                )));
            }
            return link_information;
        };
        // if let Err(error) = tab.wait_for_element_with_custom_timeout(
        //     "body > section > div > section > header > h2",
        //     Duration::from_secs(10),
        // ) {
        //     let _ = tab.reload(false, None);
        //     if let Err(error) = tab.wait_for_element_with_custom_timeout(
        //         "body > section > div > section > header > h2",
        //         Duration::from_secs(10),
        //     ) {
        //         eprintln!("Error waiting for element: {}", error);
        //         panic!(); // TODO: Return error
        //     }
        // };

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
        let selector = match Selector::parse("h2") {
            Ok(selector) => selector,
            Err(error) => {
                link_information.status = Err(LinkError::BrowserError(format!(
                    "Failed to parse selector `h2`: {}",
                    error
                )));
                return link_information;
            }
        };

        let heading = match document.select(&selector).next() {
            Some(element) => match element.text().next() {
                Some(text) => text,
                None => {
                    link_information.status = Err(LinkError::BrowserError(
                        "Failed to get heading text".to_string(),
                    ));
                    return link_information;
                }
            },
            None => {
                link_information.status =
                    Err(LinkError::BrowserError("Failed to get heading".to_string()));
                return link_information;
            }
        };

        let (filename, size, size_unit) = Self::parse_heading(heading);
        let direct_links = match Self::grab_direct_links(&document) {
            Ok(links) => links,
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

impl MultiUpMirrorLink {
    pub fn new(original_link: String, id: String) -> Self {
        Self { original_link, id }
    }

    pub fn return_mirror_link(&self) -> String {
        format!("https://multiup.io/en/mirror/{}", self.id)
    }

    fn parse_heading(heading: &str) -> (String, f32, String) {
        let parts: Vec<&str> = heading.split_whitespace().collect();
        let title = parts[3..parts.len() - 4].join(" ");
        let size = parts[parts.len() - 3].parse::<f32>().unwrap_or_default();
        let unit = parts[parts.len() - 2].to_uppercase();

        return (title, size, unit);
    }

    fn grab_direct_links(html: &Html) -> Result<Vec<DirectLink>, LinkError> {
        let mut direct_links = Vec::new();
        let selector = Selector::parse("button[type='submit'], a.host").map_err(|error| {
            LinkError::BrowserError(format!(
                "Failed to parse selector `button[type='submit'], a.host`: {}",
                error
            ))
        })?;

        let buttons: Vec<&Element> = html
            .select(&selector)
            .map(|button| button.value())
            .collect();

        for button in buttons {
            let host = button
                .attr("namehost")
                .ok_or_else(|| {
                    LinkError::BrowserError("Failed to get host from button".to_string())
                })?
                .to_owned();

            let link = button
                .attr("link")
                .ok_or_else(|| {
                    LinkError::BrowserError("Failed to get link from button".to_string())
                })?
                .to_owned();

            if link.starts_with("/") {
                continue;
            }

            let validity = button.attr("validity").ok_or_else(|| {
                LinkError::BrowserError("Failed to get validity from button".to_string())
            })?;
            let direct_link_validity = match validity {
                "valid" => DirectLinkValidity::Valid,
                "invalid" => DirectLinkValidity::Invalid,
                _ => DirectLinkValidity::Unknown,
            };
            direct_links.push(DirectLink::new(host, link, direct_link_validity));
        }

        Ok(direct_links)
    }
}

pub fn new_multiup_link(original_link: String, slug: String) -> HostingServiceLink {
    let (id, is_project) = process_slug(&slug);
    if is_project {
        HostingServiceLink::MultiUpProject(MultiUpProjectLink::new(original_link, id))
    } else {
        HostingServiceLink::MultiUpMirror(MultiUpMirrorLink::new(original_link, id))
    }
}

fn process_slug(slug: &str) -> (String, bool) {
    let mut id = String::new();
    let mut is_project = false;
    for path in slug.split('/') {
        if path.len() == 32 {
            path.clone_into(&mut id);
        } else if path == "project" {
            is_project = true;
        }
    }

    return (id, is_project);
}

// Tests
#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::modules::hosting_services::browser::{
        navigate_to_new_tab, new_browser, wait_for_element,
    };
    use crate::modules::hosting_services::links::{DirectLink, DirectLinkValidity};
    use crate::modules::hosting_services::multiup::links::{
        process_slug, MultiUpMirrorLink, MultiUpProjectLink,
    };
    use scraper::{Html, Selector};
    use DirectLinkValidity::Valid;

    #[test]
    fn test_process_slug() {
        // Short download link
        let slug = "84fdbaaa77da3e18106a3f788e1d8c22";
        let (id, is_project) = process_slug(slug);
        assert_eq!(id, "84fdbaaa77da3e18106a3f788e1d8c22");
        assert!(!is_project);

        // Long download link
        let slug = "download/84fdbaaa77da3e18106a3f788e1d8c22/Abc_news_radio_intro.mid";
        let (id, is_project) = process_slug(slug);
        assert_eq!(id, "84fdbaaa77da3e18106a3f788e1d8c22");
        assert!(!is_project);

        // Project link
        let slug = "project/170a7f224e7d146c7116dc377825a38c";
        let (id, is_project) = process_slug(slug);
        assert_eq!(id, "170a7f224e7d146c7116dc377825a38c");
        assert!(is_project);
    }

    #[tokio::test]
    #[ignore]
    async fn test_project_get_mirror_links() {
        let browser = new_browser();
        let project_link = MultiUpProjectLink::new(
            "https://multiup.io/en/project/170a7f224e7d146c7116dc377825a38c".to_owned(),
            "170a7f224e7d146c7116dc377825a38c".to_string(),
        );

        let mirror_links = project_link
            .get_mirror_links(&browser)
            .expect("Expected mirror links");
        let mirror_links_ids: Vec<&str> =
            mirror_links.iter().map(|link| link.id.as_str()).collect();
        let known_mirror_links_ids = vec![
            "84fdbaaa77da3e18106a3f788e1d8c22",
            "40df3eccaeb510058ddb33174371fc7a",
            "b48e3657caa3d4515dd9fc198ab81569",
        ];

        assert_eq!(mirror_links_ids, known_mirror_links_ids);
    }

    #[tokio::test]
    #[ignore]
    async fn test_invalid_mirror() {
        let browser = new_browser();
        let mirror_link = MultiUpMirrorLink::new(
            "https://multiup.io/en/mirror/84fdbaaa77da3e18106a3f788e1d8c21".to_owned(),
            "84fdbaaa77da3e18106a3f788e1d8c21".to_string(),
        );

        let tab = navigate_to_new_tab(&browser, mirror_link.return_mirror_link().as_str());
        let _ = wait_for_element(&tab, "h2", Some(20), Some(5));
        if let Ok(warning) =
            tab.find_element("body > section > div > section > div:nth-child(2) > div > div")
        {
            let text = warning
                .find_element("strong")
                .expect("Expected warning text element")
                .get_inner_text()
                .expect("Expected text");
            assert_eq!("The requested file could not be found", text);
        };
    }

    #[test]
    fn test_parse_heading() {
        let heading = " / Mirror list Abc_news_radio_intro.mid ( 5.60 kB )";
        let (title, size, unit) = MultiUpMirrorLink::parse_heading(heading);
        assert_eq!(title, "Abc_news_radio_intro.mid");
        assert_eq!(size, 5.60);
        assert_eq!(unit, "KB");
    }

    #[test]
    #[ignore]
    fn test_grab_direct_links() {
        // Requires human interaction for Cloudflare
        let browser = new_browser();

        let tab = browser.new_tab().unwrap();
        tab.navigate_to("https://multiup.io/en/mirror/84fdbaaa77da3e18106a3f788e1d8c22")
            .unwrap();
        let _ = tab.wait_for_element_with_custom_timeout("h2", Duration::from_secs(30));
        let page_html = tab.get_content().unwrap();

        let _ = tab.close(true).unwrap();

        let document = Html::parse_document(&page_html);
        let selector = Selector::parse("h2").unwrap();
        let direct_links =
            MultiUpMirrorLink::grab_direct_links(&document).expect("Expected direct links");
        assert_eq!(direct_links.len(), 1);
        let known_direct_link = DirectLink::new(
            "1fichier.com".to_owned(),
            "https://1fichier.com/?5gzj5zubsqqt18y1s7gh&af=62851".to_owned(),
            Valid,
        );
        assert_eq!(direct_links[0].link, known_direct_link.link);
    }
}
