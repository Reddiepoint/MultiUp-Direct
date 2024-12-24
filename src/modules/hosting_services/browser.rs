use std::ffi::OsStr;
use std::sync::Arc;
use std::time::Duration;
use eframe::egui::WidgetType::Link;
use headless_chrome::{Browser, Element, LaunchOptionsBuilder, Tab};

use crate::modules::hosting_services::links::LinkError;

pub fn new_browser() -> Browser {
    let launch_options = LaunchOptionsBuilder::default()
        .headless(false)
        .sandbox(true)
        .devtools(false)
        .idle_browser_timeout(Duration::from_secs(1000))
        .ignore_default_args({
            let args = vec!["--enable-automation"];
            args.into_iter().map(OsStr::new).collect()
        })
        .build()
        .unwrap();
    let browser = match Browser::new(launch_options) {
        Ok(browser) => browser,
        Err(error) => panic!("An error occurred: {}", error),
    };
    browser
}

pub fn navigate_to_new_tab(browser: &Browser, url: &str) -> Result<Arc<Tab>, (Option<Arc<Tab>>, LinkError)> {
    let tab = match browser.new_tab() {
        Ok(tab) => tab,
        Err(error) => {
            return Err((None, LinkError::BrowserError(format!("Failed to create new tab: {}", error))));
        }
    };

    if let Err(error) = tab.navigate_to(url) {
        return Err((Some(tab), LinkError::BrowserError(format!("Failed to navigate to URL {}: {}", url, error))));
    }

    if let Err(error) = tab.wait_until_navigated() {
        return Err((Some(tab), LinkError::BrowserError(format!("Failed to navigate to URL {}: {}", url, error))));
    };

    Ok(tab)
}

/// Waits for an element to appear on the page.
///
/// Duration is in seconds (default is 10 seconds).
/// On retry, the tab will refresh the page and wait for the element (default is 0).
pub fn wait_for_element<'a>(
    tab: &'a Arc<Tab>,
    selector: &'a str,
    duration: Option<usize>,
    retries: Option<usize>,
) -> Result<Element<'a>, LinkError> {
    let mut total_retries = 0;
    let timeout = Duration::from_secs(duration.unwrap_or(10) as u64);
    let mut error_message = None;
    while total_retries <= retries.unwrap_or(0) {
        match tab.wait_for_element_with_custom_timeout(selector, timeout) {
            Ok(element) => {
                return Ok(element);
            }
            Err(error) => {
                let _ = tab.reload(false, None);
                error_message = Some(error.to_string());
                total_retries += 1;
            }
        }
    }

    Err(LinkError::TimedOut(
        format!(
            "Error waiting for element `{}`: {}",
            selector,
            error_message.unwrap_or("Unknown error".to_string())
        ),
        duration.unwrap_or(10) as u64,
    ))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use headless_chrome::Tab;
    use crate::modules::hosting_services::browser::{navigate_to_new_tab, new_browser};
    use crate::modules::hosting_services::links::LinkError;

    #[test]
    #[ignore]
    fn test_navigate_to_new_tab() {
        let browser = new_browser();
        let _tab = match navigate_to_new_tab(&browser, "https://google.com") {
            Ok(_) => {}
            Err(_) => panic!("Failed to navigate to tab")
        };
    }
}
