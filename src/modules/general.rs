use std::ffi::OsStr;
use std::time::Duration;

use async_recursion::async_recursion;
use crossbeam_channel::{Receiver, TryRecvError};
use headless_chrome::{Browser, LaunchOptions};
use reqwest::Client;

use crate::modules::links::LinkError;

#[async_recursion]
pub async fn get_page_html(
    page_link: &str,
    client: &Client,
    cancel_receiver: Option<Receiver<bool>>,
    try_count: u8,
    browser: Browser
) -> Result<String, LinkError> {
    if let Some(receiver) = cancel_receiver.clone() {
        if let Ok(_) | Err(TryRecvError::Disconnected) = receiver.try_recv() {
            return Err(LinkError::Cancelled);
        }
    }

    if try_count >= 10 {
        return Err(LinkError::TimedOut);
    }


    let mut tab = browser.new_tab().unwrap();
    // let tab = browser.new_context().unwrap().new_tab().unwrap();
    // tab.set_user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36 Edg/122.0.0.",
    //                    None, Some("Windows")).unwrap();
    // println!("Navigating to {}", page_link);
    // tab.enable_stealth_mode().unwrap();
    tab.navigate_to(page_link).unwrap();

    // let cookies = tab.get_cookies().unwrap().iter().map(|cookie| DeleteCookies {
    //     name: cookie.name.to_string(),
    //     url: Some(page_link.to_string()),
    //     domain: Some(cookie.domain.to_string()),
    //     path: Some(cookie.path.to_string()),
    // }).collect();
    // tab.delete_cookies(cookies).unwrap();
    match tab.wait_for_element_with_custom_timeout("body > section > div > section > header > h2", Duration::from_secs(60)) {
        Ok(_) => {}
        Err(_) => {
            // println!("ERROR HTML: {}", tab.get_content().unwrap());
        }
    }

    return match tab.get_content() {
        Ok(html) => {
            // println!("HTML: {}", html);
            tab.close(true).unwrap();
            Ok(html)
        }
        Err(error) => {
            // eprintln!("{}", error);
            Err(LinkError::Other)
        }
    };

    // let server_response = match client.get(page_link).send().await {
    //     Ok(response) => response,
    //     Err(error) => return Err(LinkError::Reqwest(error))
    // };

    // return match server_response.error_for_status() {
    //     Ok(response) => Ok(response.text().await.unwrap().to_string()),
    //     Err(error) => {
    //         // Repeat if error is not 404
    //         if error.status().unwrap() != StatusCode::NOT_FOUND {
    //             println!("{}", error);
    //             let _ = tokio::time::sleep(Duration::from_millis(100)).await;
    //             return get_page_html(page_link, client, cancel_receiver, try_count + 1).await;
    //         }
    //         Err(LinkError::Invalid)
    //     }
    // };
}

pub fn new_browser() -> Browser {
    let browser = Browser::new(
        LaunchOptions {
            headless: false,
            sandbox: false,
            enable_gpu: false,
            enable_logging: false,
            idle_browser_timeout: Duration::from_secs(60),
            window_size: None,
            path: None,
            user_data_dir: None,
            port: None,
            ignore_certificate_errors: true,
            extensions: Vec::new(),
            process_envs: None,
            // #[cfg(feature = "fetch")]
            // fetcher_options: Default::default(),
            args: {
                let args = [
                    // "--disable-background-networking",
                    "--enable-features=NetworkService,NetworkServiceInProcess",
                    "--disable-background-timer-throttling",
                    // "--disable-backgrounding-occluded-windows",
                    "--disable-breakpad",
                    "--disable-client-side-phishing-detection",
                    "--disable-component-extensions-with-background-pages",
                    "--disable-default-apps",
                    "--disable-dev-shm-usage",
                    "--disable-extensions",
                    // BlinkGenPropertyTrees disabled due to crbug.com/937609
                    "--disable-features=TranslateUI,BlinkGenPropertyTrees",
                    "--disable-hang-monitor",
                    "--disable-ipc-flooding-protection",
                    "--disable-popup-blocking",
                    "--disable-prompt-on-repost",
                    // "--disable-renderer-backgrounding",
                    // "--disable-sync",
                    // "--force-color-profile=srgb",
                    // "--metrics-recording-only",
                    "--no-first-run",
                    // "--enable-automation",
                    "--password-store=basic",
                    "--use-mock-keychain",
                ];
                args.clone().iter().map(OsStr::new).collect()
            },
            disable_default_args: true,
            proxy_server: None,
        }
    ).unwrap();
    return browser;
}
