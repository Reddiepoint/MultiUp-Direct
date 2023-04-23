


pub async fn get_link_hosts(url: &String) -> Result<String, reqwest::Error> {
    let url = "https://multiup.org/en/mirror/b4804b9d945410c6a5990acc2691d2ad/test.txt";
    let website_html = reqwest::get(url).await?.text().await?;
    let website_html = scraper::Html::parse_document(&website_html);
    let button_selector = scraper::Selector::parse(r#"button[type="submit"]"#).unwrap();
    for element in website_html.select(&button_selector) {
        let namehost = element.value().attr("namehost").unwrap();
        let link = element.value().attr("link").unwrap();
        println!("namehost: {}, link: {}", namehost, link);
    };
    Ok(String::new())
}