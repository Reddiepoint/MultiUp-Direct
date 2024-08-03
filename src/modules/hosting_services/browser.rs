use std::ffi::OsStr;
use std::sync::Arc;
use std::time::Duration;

use headless_chrome::{Browser, Element, LaunchOptionsBuilder, Tab};

use crate::modules::hosting_services::links::LinkError;

pub fn new_browser() -> Browser {
    let launch_options = LaunchOptionsBuilder::default()
        .headless(false)
        .sandbox(true)
        .disable_default_args(true)
        .args({
            let args = vec![
                "--enable-features=NetworkService,NetworkServiceInProcess",
                "--disable-background-timer-throttling",
                "--disable-backgrounding-occluded-windows",
                "--disable-breakpad",
                "--disable-default-apps",
                "--disable-dev-shm-usage",
                "--disable-extensions",
                // BlinkGenPropertyTrees disabled due to crbug.com/937609
                "--disable-features=TranslateUI,BlinkGenPropertyTrees",
                "--disable-hang-monitor",
                "--disable-ipc-flooding-protection",
                "--disable-popup-blocking",
                "--disable-prompt-on-repost",
                "--disable-renderer-backgrounding",
                "--disable-sync",
                "--force-color-profile=srgb",
                "--metrics-recording-only",
                "--no-first-run",
                // "--enable-automation",
                "--password-store=basic",
                "--use-mock-keychain",
                // "--no-startup-window",
            ];
            args.into_iter().map(OsStr::new).collect()
        })
        .build()
        .unwrap();
    let browser = match Browser::new(launch_options) {
        Ok(browser) => browser,
        Err(error) => panic!("An error occurred: {}", error),
    };
    return browser;
}

pub fn navigate_to_new_tab(browser: &Browser, url: &str) -> Arc<Tab> {
    let tab = browser.new_tab().unwrap();
    tab.navigate_to(url).unwrap();
    return tab;
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
        println!("Waiting");
        match tab.wait_for_element_with_custom_timeout(selector, timeout) {
            Ok(element) => {
                return Ok(element);
            }
            Err(error) => {
                total_retries += 1;
                let _ = tab.reload(false, None);
                error_message = Some(error)
            }
        }
    }
    eprintln!("Error waiting for element: {}", error_message.unwrap());
    panic!(); // TODO: Return error
}

#[cfg(test)]
mod tests {
    use crate::modules::hosting_services::browser::{navigate_to_new_tab, new_browser};

    #[test]
    fn test_navigate_to_new_tab() {
        let browser = new_browser();
        let tab = navigate_to_new_tab(&browser, "https://google.com");
    }
}
