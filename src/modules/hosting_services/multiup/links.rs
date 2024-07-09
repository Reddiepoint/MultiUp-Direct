use std::time::Duration;

use headless_chrome::Browser;
use scraper::node::Element;
use scraper::selectable::Selectable;
use scraper::{Html, Selector};

use crate::modules::hosting_services::extract::recognise_links;
use crate::modules::hosting_services::links::{
    DirectLink, HostingService, HostingServiceLink, LinkInformation,
};

pub struct MultiUpProjectLink {
    original_link: String,
    id: String,
}

impl HostingService for MultiUpProjectLink {
    fn get_id(&self) -> &str {
        &self.id
    }

    fn get_domain(&self) -> &str {
        "multiup.io"
    }

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

    pub async fn get_mirror_links(&self, browser: &Browser) -> Vec<MultiUpMirrorLink> {
        let tab = browser.new_tab().unwrap();
        let _ = tab.navigate_to(&self.return_project_link());
        match tab
            .wait_for_element_with_custom_timeout("#textarea-links-long", Duration::from_secs(10))
        {
            Ok(_) => {}
            Err(error) => {
                eprintln!("Error waiting for element: {}", error);
                panic!()
            }
        };
        let page_html = match tab.get_content() {
            Ok(html) => html,
            Err(error) => {
                eprintln!("Error getting content: {}", error);
                panic!(); // TODO: Return error
            }
        };

        tab.close(false).unwrap();

        let document = Html::parse_document(&page_html);
        let selector = Selector::parse("#textarea-links-long").unwrap();
        let mirror_links = document
            .select(&selector)
            .next()
            .unwrap()
            .text()
            .next()
            .unwrap();

        let links = recognise_links(mirror_links);

        links
            .into_iter()
            .map(|link| match link {
                HostingServiceLink::MultiUpMirror(link) => link,
                _ => unreachable!(),
            })
            .collect()
    }
}

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
            String::new(),
            0.0,
            String::new(),
            Vec::new(),
            Ok(()),
        );
        let tab = browser.new_tab().unwrap();
        tab.navigate_to(&self.return_mirror_link()).unwrap();
        if let Err(error) = tab.wait_for_element_with_custom_timeout(
            "body > section > div > section > header > h2",
            Duration::from_secs(30),
        ) {
            let _ = tab.reload(false, None);
            if let Err(error) = tab.wait_for_element_with_custom_timeout(
                "body > section > div > section > header > h2",
                Duration::from_secs(10),
            ) {
                eprintln!("Error waiting for element: {}", error);
                panic!(); // TODO: Return error
            }
        };

        let page_html = match tab.get_content() {
            Ok(html) => html,
            Err(error) => {
                eprintln!("Error getting content: {}", error);
                panic!();
            }
        };

        let _ = tab.close(false).unwrap();

        let document = Html::parse_document(&page_html);
        let selector = Selector::parse("body > section > div > section > header > h2").unwrap();
        let heading = document
            .select(&selector)
            .next()
            .unwrap()
            .text()
            .next()
            .unwrap();
        let (filename, size, size_unit) = Self::parse_heading(heading);
        let direct_links = Self::grab_direct_links(&document);
        link_information.filename = filename;
        link_information.size = size;
        link_information.size_unit = size_unit;
        link_information.direct_links = direct_links;
        return link_information;
        
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
        let size = parts[parts.len() - 3].parse::<f32>().unwrap();
        let unit = parts[parts.len() - 2].to_uppercase();

        return (title, size, unit);
    }

    fn grab_direct_links(html: &Html) -> Vec<DirectLink> {
        let mut direct_links = Vec::new();
        let selector = Selector::parse("button[type='submit']").unwrap();

        let buttons: Vec<&Element> = html
            .select(&selector)
            .map(|button| button.value())
            .collect();

        for button in buttons {
            let host = button.attr("namehost").unwrap().to_owned();
            let link = button.attr("link").unwrap().to_owned();
            let validity = button.attr("validity").unwrap().to_owned();
            direct_links.push(DirectLink::new(host, link, validity));
        }

        return direct_links;
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

    use scraper::{Html, Selector};

    use crate::modules::hosting_services::browser::new_browser;
    use crate::modules::hosting_services::links::DirectLink;
    use crate::modules::hosting_services::multiup::links::{
        process_slug, MultiUpMirrorLink, MultiUpProjectLink,
    };

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
    async fn test_project_get_mirror_links() {
        let browser = new_browser();
        let project_link = MultiUpProjectLink::new(
            "https://multiup.io/en/project/170a7f224e7d146c7116dc377825a38c".to_owned(),
            "170a7f224e7d146c7116dc377825a38c".to_string(),
        );

        let mirror_links = project_link.get_mirror_links(&browser).await;
        let mirror_links_ids: Vec<&str> =
            mirror_links.iter().map(|link| link.id.as_str()).collect();
        let known_mirror_links_ids = vec![
            "84fdbaaa77da3e18106a3f788e1d8c22",
            "40df3eccaeb510058ddb33174371fc7a",
            "b48e3657caa3d4515dd9fc198ab81569",
        ];

        assert_eq!(mirror_links_ids, known_mirror_links_ids);
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
    fn test_grab_direct_links() {
        let browser = new_browser();

        let tab = browser.new_tab().unwrap();
        tab.navigate_to("https://multiup.io/en/mirror/84fdbaaa77da3e18106a3f788e1d8c22")
            .unwrap();
        let _ = tab.wait_for_element_with_custom_timeout(
            "body > section > div > section > header > h2",
            Duration::from_secs(30),
        );
        let page_html = tab.get_content().unwrap();

        let _ = tab.close(false).unwrap();

        let document = Html::parse_document(&page_html);
        let selector = Selector::parse("body > section > div > section > header > h2").unwrap();
        let direct_links = MultiUpMirrorLink::grab_direct_links(&document);
        assert_eq!(direct_links.len(), 1);
        let known_direct_link = DirectLink::new(
            "1fichier.com".to_owned(),
            "https://1fichier.com/?5gzj5zubsqqt18y1s7gh&af=62851".to_owned(),
            "valid".to_owned(),
        );
        assert_eq!(direct_links[0].link, known_direct_link.link);
    }
}
