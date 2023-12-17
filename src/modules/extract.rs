use std::collections::{BTreeSet, HashSet};
use async_recursion::async_recursion;
use eframe::egui::{Align, Button, Layout, ScrollArea, TextEdit, Ui};
use egui_extras::{Column, TableBuilder};
use regex::Regex;
use reqwest::{Client, StatusCode};
use scraper::{ElementRef, Selector};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
use scraper::node::Element;
use tokio::io::split;
use tokio::runtime::Runtime;
use tokio::sync::Semaphore;
use crate::modules::links::{DirectLink, DownloadLink, LinkError, MultiUpLink, MultiUpLinkInformation, ProjectLink};

#[derive(Default)]
pub struct ExtractUI {
    multiup_links: String,
    recheck_validity: bool,
    currently_extracting: bool,
    cancelled_extraction: bool,
}

impl ExtractUI {
    pub fn display(ui: &mut Ui, extract_ui: &mut ExtractUI) {
        extract_ui.display_input_area(ui);
        extract_ui.display_link_information(ui);
        extract_ui.display_output_area(ui);
    }

    fn display_input_area(&mut self, ui: &mut Ui) {
        ui.heading("MultiUp Links");

        let input_area_height = ui.available_height() / 4.0;
        ui.set_max_height(input_area_height);
        ScrollArea::both()
            .id_source("MultiUp Link Input Area")
            .max_height(input_area_height)
            .show(ui, |ui| {
                ui.add(
                    TextEdit::multiline(&mut self.multiup_links)
                        .hint_text("Paste your MultiUp links here")
                        .desired_width(ui.available_width()),
                );
            });

        // UI elements related to the extraction of links
        ui.horizontal(|ui| {
            // Recheck validity checkbox
            ui.checkbox(&mut self.recheck_validity, "Recheck link validity");

            // Extract links button
            if ui
                .add_enabled(!self.currently_extracting, Button::new("Extract links"))
                .clicked()
            {
                // Main extraction function
                let rt = Runtime::new().unwrap();
                let multiup_links = self.multiup_links.clone();
                let recheck_validity = self.recheck_validity;
                thread::spawn(move || {
                    rt.block_on(async {
                        extract_direct_links(&multiup_links, recheck_validity).await;
                    });
                });
            }

            // Generation text and cancel extraction button
            if self.currently_extracting && !self.cancelled_extraction {
                ui.spinner();
                ui.label("Extracting links...");
                if ui.button("Cancel now").clicked() {}
            }
        });
    }

    fn display_link_information(&mut self, ui: &mut Ui) {
        ui.collapsing("Link Information", |ui| {
            let width = ui.available_width();
            TableBuilder::new(ui)
                // Column for selecting which MultiUp links to show
                .column(Column::auto())
                // Column for MultiUp link information
                .column(Column::remainder())
                .cell_layout(Layout::left_to_right(Align::Center))
                .body(|body| {});
        });
    }

    fn display_output_area(&mut self, ui: &mut Ui) {
        ui.heading("Direct Links");

        ui.horizontal(|ui| {
            let output_box_width = 0.75 * ui.available_width();
            TableBuilder::new(ui)
                .column(Column::exact(output_box_width))
                .body(|body| {});

            self.display_filter_menu_area(ui);
        });
    }

    fn display_filter_menu_area(&mut self, ui: &mut Ui) {}
}

// Extraction Functions
async fn extract_direct_links(input_text: &str, recheck_validity: bool) {
    // Detect links
    let detected_links = detect_links(input_text);

    // Process links

    let processed_links = process_links(detected_links).await;


    // Return vec of mirror links
    let time_now = Instant::now();
    get_direct_links(processed_links, recheck_validity).await;
    let time_taken = time_now.elapsed();
    println!("{}", time_taken.as_secs_f32());
}

/// Detects MultiUp links in the given input text.
fn detect_links(input_text: &str) -> Vec<String> {
    // Create regexes
    let (multiup_regex, _, _, _) = create_regexes();
    // Pre-allocate memory for a vec which contains all detected MultiUp links
    let mut detected_links: Vec<String> = Vec::with_capacity(input_text.lines().count());

    // Detection
    for captures in multiup_regex.captures_iter(input_text) {
        let link = captures[0].to_string();
        detected_links.push(link);
    }
        
    // Return detected links
    detected_links
}

async fn process_links(detected_links: Vec<String>) -> Vec<MultiUpLink> {
    // Create regexes
    let (_, download_regex, mirror_regex, project_regex) = create_regexes();

    // Pre-allocate memory for a vec which contains all detected MultiUp links
    // Follows the system of Vec<(original_link, id, name, is_project, status)>
    let mut processed_links: Vec<MultiUpLink> = Vec::with_capacity(detected_links.len());

    // Store tasks for processing project links, which will be awaited after other links are processed
    let mut project_processing_tasks = Vec::new();

    // Processing
    for link in detected_links {
        if project_regex.is_match(&link) {
            let link = link.clone();
            let processing_task = tokio::spawn(async move {
                process_project_link(&link).await
            });
            project_processing_tasks.push(processing_task);
        } else if mirror_regex.is_match(&link) {
            let download_link= MultiUpLink::Download(process_non_project_link(&link.clone(), &mirror_regex));
            if !processed_links.contains(&download_link) {
                processed_links.push(download_link);
            }

        } else if download_regex.is_match(&link) {
            let download_link= MultiUpLink::Download(process_non_project_link(&link.clone(), &download_regex));
            if !processed_links.contains(&download_link) {
                processed_links.push(download_link);
            }
        }
    }

    let project_links = futures::future::join_all(project_processing_tasks).await;
    for link in project_links {
        processed_links.append(&mut vec![link.unwrap()])
    }

    // println!("{:?}", processed_links);
    processed_links
}
static DOWNLOAD_REGEX: OnceLock<Regex> = OnceLock::new();
static MIRROR_REGEX: OnceLock<Regex> = OnceLock::new();
static PROJECT_REGEX: OnceLock<Regex> = OnceLock::new();
static MULTIUP_REGEX: OnceLock<Regex> = OnceLock::new();

/// Creates and initialises regular expressions used for matching different types of links.
///
/// Returns a tuple containing four regular expressions:
/// - `multiup_regex`: Matches all MultiUp links.
/// - `download_regex`: Matches download links.
/// - `mirror_regex`: Matches mirror links.
/// - `project_regex`: Matches project links.
pub fn create_regexes() -> (Regex, Regex, Regex, Regex) {
    // All MultiUp links
    let multiup_regex = MULTIUP_REGEX
        .get_or_init(|| Regex::new(r"(https?://(www\.)?multiup\.(org|io)/\S+)").unwrap());

    // Download links
    let download_regex = DOWNLOAD_REGEX.get_or_init(|| {
        Regex::new(r"https?://(www\.)?multiup\.(org|io)/(en/)?(download/)?").unwrap()
    });

    // Mirror links
    let mirror_regex = MIRROR_REGEX.get_or_init(|| {
        Regex::new(r"https?://multiup\.(org|io)/en/mirror/").unwrap()
    });

    // Project links
    let project_regex = PROJECT_REGEX.get_or_init(|| {
        Regex::new(r"^https://(www\.)?multiup\.(org|io)/(en/)?project/.*$").unwrap()
    });

    (
        multiup_regex.to_owned(),
        download_regex.to_owned(),
        mirror_regex.to_owned(),
        project_regex.to_owned(),
    )
}

/// Processes a given project link.
///
/// This function takes in a project link, mirror regex, and download regex as inputs,
/// and returns a Project MultiUpLink.
async fn process_project_link(project_link: &str,) -> MultiUpLink {
    // Download links
    let download_regex = DOWNLOAD_REGEX.get().unwrap();

    // Mirror links
    let mirror_regex = MIRROR_REGEX.get().unwrap();

    let (id, name, download_links) = get_project_information(project_link).await;
    let download_links = match download_links {
        Ok(download_links) => download_links,
        Err(error) => {
            let mut project_link = ProjectLink::new(project_link.to_string(), id, name);
            project_link.status = Some(Err(error));
            return MultiUpLink::Project(project_link);
        }

    };
    // let download_links = get_project_download_links(project_link).await?;
    let mut processed_links: HashSet<DownloadLink> = HashSet::with_capacity(download_links.len());

    for link in download_links {
        if mirror_regex.is_match(&link) {
            let download_link= process_non_project_link(&link.clone(), &mirror_regex);
            processed_links.insert(download_link);

        } else if download_regex.is_match(&link) {
            let download_link= process_non_project_link(&link.clone(), &download_regex);
            processed_links.insert(download_link);
        }
    }

    let mut project_link = ProjectLink::new(project_link.to_string(), id, name);
    project_link.download_links = Some(processed_links);
    project_link.status = Some(Ok(()));
    MultiUpLink::Project(project_link)
}

static PROJECT_DOWNLOAD_LINKS_SELECTOR: OnceLock<Selector> = OnceLock::new();
static PROJECT_TITLE_SELECTOR: OnceLock<Selector> = OnceLock::new();
/// Retrieves information about a project given a project link.
///
/// Parses the project link for an ID, parses the page title for a name and extracts download links.
/// If there is no name, it is set to the ID.
#[async_recursion]
async fn get_project_information(project_link: &str) -> (String, String, Result<Vec<String>, LinkError>) {
    let link_parts: Vec<&str> = project_link.split('/').collect();
    let id = link_parts.last().unwrap().to_string();
    let name = id.clone();

    let client = Client::new();
    let server_response = match client.get(project_link).send().await {
        Ok(response) => response,
        Err(error) => return (id, name, Err(LinkError::Reqwest(error))),
    };

    let html = match server_response.error_for_status() {
        Ok(res) => res.text().await.unwrap().to_string(),
        Err(error) => {
            // Repeat if error is not 404, otherwise, return invalid
            if error.status().unwrap() != StatusCode::NOT_FOUND {
                let _ = tokio::time::sleep(Duration::from_millis(100)).await;
                return get_project_information(project_link).await;
            }
            return (id, name, Err(LinkError::Invalid));
        }
    };

    let parsed_page = scraper::Html::parse_document(&html);

    let project_title_selector = PROJECT_TITLE_SELECTOR
        .get_or_init(|| Selector::parse(r#".text-truncate"#).unwrap());
    let name = match parsed_page.select(project_title_selector).next() {
        Some(title) => {
            let title_text = title.text().last().unwrap().to_string();
            match get_project_name_from_title(&title_text) {
                Some(name) => name.to_string(),
                None => id.clone()
            }
        },
        None => id.clone()
    };

    let project_download_links_selector = PROJECT_DOWNLOAD_LINKS_SELECTOR
        .get_or_init(|| Selector::parse(r#"#textarea-links-long"#).unwrap());
    let links = match parsed_page.select(project_download_links_selector).next() {
        Some(links) => {
            Ok(links.inner_html().to_string().split('\n').map(|link| link.to_string()).collect())
        },
        None => return (id, name, Err(LinkError::NoLinks)),
    };
    (id, name, links)
}

/// Extracts the project name from a given title text.
fn get_project_name_from_title(title_text: &str) -> Option<&str> {
    let prefix = " / Project ";
    if let Some(index) = title_text.find(prefix) {
        let name_start = index + prefix.len();
        let name_end = title_text.find(" (").unwrap_or(title_text.len());
        let name = &title_text[name_start..name_end];

        Some(name)
    } else {
        None
    }
}

/// Process a non-project link and return a `DownloadLink` object.
fn process_non_project_link(link: &str, regex: &Regex) -> DownloadLink {
    let link_parts = regex.replace(link, "");
    let mut link_parts = link_parts.split('/');
    let id = link_parts.next().unwrap().to_string();

    DownloadLink::new(link.to_string(), id)
}


async fn get_direct_links(mut multiup_links: Vec<MultiUpLink>, recheck_validity: bool) {
    // At the beginning of the function
    let semaphore = Arc::new(Semaphore::new(5));
    let mut tasks = Vec::new();
    println!("{:?}", multiup_links);
    for link in multiup_links {
        match link {
            MultiUpLink::Project(mut project_link) => {
                // Create a task for each project link
                let semaphore = Arc::clone(&semaphore);
                let task = tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    get_direct_links_from_project(project_link, recheck_validity).await
                });
                tasks.push(task);
            }
            MultiUpLink::Download(mut download_link) => {
                // Create a task for each download link
                let semaphore = Arc::clone(&semaphore);
                let task = tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    get_direct_links_from_download_link(download_link, recheck_validity).await
                });
                tasks.push(task);
            }
        }
    }

// Wait for all tasks to complete
    let results = futures::future::join_all(tasks).await;

// Sort the results based on their order of processing
//     results.sort_by_key(|(order, _)| *order);

// Extract the direct links from the results
//     let direct_links: Vec<(usize, MirrorLink)> = results.into_iter().map(|(_, result)| result).collect();
}

async fn get_direct_links_from_project(mut project_link: ProjectLink, recheck_validity: bool) {

}

const MIRROR_PREFIX: &str = "https://multiup.io/en/mirror/";
async fn get_direct_links_from_download_link(mut download_link: DownloadLink, recheck_validity: bool) {
    let mirror_link = MIRROR_PREFIX.to_owned() + &download_link.link_id + "/dummy_text";
    println!("{mirror_link}");
    if recheck_validity {
        recheck_validity_api(mirror_link, download_link).await;
    } else {
        process_mirror_link(mirror_link, download_link).await;
    }
}

async fn recheck_validity_api(mirror_link: String, mut download_link: DownloadLink) {

}

async fn process_mirror_link(mirror_link: String, mut download_link: DownloadLink) {
    let _ = get_mirror_information(&mirror_link).await;


}

static MIRROR_HOSTS_SELECTOR: OnceLock<Selector> = OnceLock::new();
static MIRROR_TITLE_SELECTOR: OnceLock<Selector> = OnceLock::new();
static QUEUE_SELECTOR: OnceLock<Selector> = OnceLock::new();
#[async_recursion]
async fn get_mirror_information(mirror_link: &str) -> Result<(BTreeSet<DirectLink>), LinkError> {
    let mut direct_links: BTreeSet<DirectLink> = BTreeSet::new();

    let client = Client::new();
    let server_response = match client.get(mirror_link).send().await {
        Ok(response) => response,
        Err(error) => return Err(LinkError::Reqwest(error)),
    };

    let html = match server_response.error_for_status() {
        Ok(res) => res.text().await.unwrap().to_string(),
        Err(error) => {
            // Repeat if error is not 404, otherwise, return invalid
            if error.status().unwrap() != StatusCode::NOT_FOUND {
                let _ = tokio::time::sleep(Duration::from_millis(100)).await;
                return get_mirror_information(mirror_link).await;
            }
            return Err(LinkError::Invalid);
        }
    };

    let parsed_page = scraper::Html::parse_document(&html);

    let mirror_hosts_selector = MIRROR_HOSTS_SELECTOR.get_or_init(|| Selector::parse(r#"a.host[namehost], button.host[namehost]"#).unwrap());
    for button in parsed_page.select(mirror_hosts_selector) {
        let direct_link = get_direct_link_from_button(button);
        direct_links.insert(direct_link);
    }
    if direct_links.is_empty() {
        return Err(LinkError::NoLinks);
    }

    let file_name_selector = MIRROR_TITLE_SELECTOR.get_or_init(|| Selector::parse(r#"h2.text-truncate"#).unwrap());
    let title = get_title_and_size_from_title_text(parsed_page.select(file_name_selector).next().unwrap());
    let link_information = MultiUpLinkInformation::new_basic(title.0, title.1);

    let queue_selector = QUEUE_SELECTOR.get_or_init(|| Selector::parse(r#"body > section > div > section > div.row > div > section > div > div > div:nth-child(2) > div > h4"#).unwrap());
    if let Some(queue_message) = parsed_page.select(queue_selector).next() {
        return Err(LinkError::InQueue);
    }

    Ok(direct_links)
}

fn get_direct_link_from_button(button: ElementRef) -> DirectLink {
    let button_value = button.value();
    let host_name = button_value.attr("namehost").unwrap();
    let link = button_value.attr("link").unwrap();
    let validity = button_value.attr("validity").unwrap();

    DirectLink::new(host_name.to_string(), link.to_string(), validity.to_string())
}

fn get_title_and_size_from_title_text(title: ElementRef) -> (String, u64) {
    let mirror_title = title.text().last().unwrap().to_string();
    // Extract the file name
    let file_name = mirror_title.trim_start_matches(" / Mirror list ").split(" (").next().unwrap();
    println!("{}", file_name);
    // Extract the size value and unit
    let size_match = mirror_title
        .trim_end_matches(" )").rsplit(" (")
        .next()
        .unwrap()
        .split_whitespace()
        .collect::<Vec<&str>>();
    let size_value = size_match[0].parse::<f64>().ok().unwrap();
    let size_unit = size_match[1].to_lowercase();


    // Convert size into bytes
    let size_in_bytes = match size_unit.as_str() {
        "kb" => (size_value * 1024.0) as u64,
        "mb" => (size_value * 1024.0 * 1024.0) as u64,
        "gb" => (size_value * 1024.0 * 1024.0 * 1024.0) as u64,
        _ => 0,
    };

    (file_name.to_string(), size_in_bytes)
}
