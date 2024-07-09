use std::ffi::OsStr;

use headless_chrome::{Browser, LaunchOptionsBuilder};

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
    let browser = Browser::new(launch_options).unwrap();
    return browser;
}

#[cfg(test)]
mod tests {
    use crate::modules::hosting_services::browser::new_browser;

    #[test]
    fn test_open_tab() {
        let browser = new_browser();
        let tab = browser.new_tab().unwrap();
        tab.navigate_to("https://google.com").unwrap();
    }
}
