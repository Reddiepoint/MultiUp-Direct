use std::time::Duration;

use headless_chrome::Browser;
use scraper::{Html, Selector};

use crate::modules::hosting_services::extract::recognise_links;
use crate::modules::hosting_services::links::{DirectLink, HostingService, HostingServiceLink};

pub struct MultiUpProjectLink {
    original_link: String,
    id: String,
}
impl MultiUpProjectLink {
    pub fn new(original_link: String, id: String) -> Self {
        Self { original_link, id }
    }

    fn return_project_link(&self) -> String {
        format!("https://multiup.io/en/project/{}", self.id)
    }
    pub async fn get_mirror_links(&self, browser: Browser) -> Vec<MultiUpMirrorLink> {
        let tab = browser.new_tab().unwrap();
        let _ = tab.navigate_to(&self.return_project_link());
        match tab
            .wait_for_element_with_custom_timeout("#textarea-links-long", Duration::from_secs(60))
        {
            Ok(_) => {}
            Err(error) => {
                eprintln!("Error waiting for element: {}", error);
                panic!()
            }
        };
        let page_html = match tab.get_content() {
            Ok(html) => {
                tab.close(false).unwrap();
                html
            }
            Err(error) => {
                eprintln!("Error getting content: {}", error);
                panic!()
            }
        };

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
    fn get_direct_links(&self, browser: Browser) -> Vec<DirectLink> {
        todo!()
    }
}
impl MultiUpMirrorLink {
    pub fn new(original_link: String, id: String) -> Self {
        Self { original_link, id }
    }

    pub fn return_mirror_link(&self) -> String {
        format!("https://multiup.io/en/mirror/{}", self.id)
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
    use crate::modules::hosting_services::browser::new_browser;
    use crate::modules::hosting_services::multiup::links::{process_slug, MultiUpProjectLink};

    #[test]
    fn test_process_slug() {
        // Short download link
        let slug = "507cf0314f5f29d404a2425db90e0dd1";
        let (id, is_project) = process_slug(slug);
        assert_eq!(id, "507cf0314f5f29d404a2425db90e0dd1");
        assert!(!is_project);

        // Long download link
        let slug = "download/507cf0314f5f29d404a2425db90e0dd1/The Night Way Home v1.065.7z";
        let (id, is_project) = process_slug(slug);
        assert_eq!(id, "507cf0314f5f29d404a2425db90e0dd1");
        assert!(!is_project);

        // Project link
        let slug = "project/1e8a98c29c6a1db357f447ed278d0594";
        let (id, is_project) = process_slug(slug);
        assert_eq!(id, "1e8a98c29c6a1db357f447ed278d0594");
        assert!(is_project);
    }

    #[tokio::test]
    async fn test_get_mirror_links() {
        let browser = new_browser();
        let project_link = MultiUpProjectLink::new(
            "https://multiup.io/en/project/1e8a98c29c6a1db357f447ed278d0594".to_owned(),
            "1e8a98c29c6a1db357f447ed278d0594".to_string(),
        );

        let mirror_links = project_link.get_mirror_links(browser).await;
        let mirror_links_ids: Vec<&str> =
            mirror_links.iter().map(|link| link.id.as_str()).collect();
        let known_mirror_links_ids = vec![
            "2a807bab69041ce057e66f020d981044",
            "d663190b183f6bdb2adc7cd3d28b719b",
            "40132c0adfd89b387db8953fcf0db98e",
        ];

        assert_eq!(mirror_links_ids, known_mirror_links_ids);
    }
}
