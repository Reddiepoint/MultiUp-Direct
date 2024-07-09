use std::ffi::OsStr;
use headless_chrome::{Browser, LaunchOptionsBuilder};

pub fn new_browser() -> Browser {
    let launch_options = LaunchOptionsBuilder::default()
        .headless(false)
        .sandbox(true)
        .args({
            let args = vec![
                "--no-first-run",
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
