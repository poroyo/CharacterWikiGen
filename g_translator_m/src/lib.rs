use anyhow::Error;
use scraper::{Html, Selector};
#[cfg(target_arch = "wasm32")] 
use shared_constants::PROXY;

async fn translate(text: String, from: &str, to: &str) -> Result<String, Error> {
    #[cfg(target_arch = "wasm32")] 
    let url = format!(
        "{}https://translate.google.com/m?tl={}&sl={}&q={}",
        PROXY, to, from, text
    );

    #[cfg(not(target_arch = "wasm32"))]
    let url = format!(
        "https://translate.google.com/m?tl={}&sl={}&q={}",
        to, from, text
    );

    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .send()
        .await?
        .text()
        .await?;

    let result = parse_document(response);

    Ok(result)
}

fn parse_document(res: String) -> String {
    let fragment = Html::parse_document(&res);
    let selector = Selector::parse(".result-container").expect("Parsing failed");
    let result = fragment
        .select(&selector)
        .next()
        .unwrap()
        .text()
        .collect::<Vec<_>>()
        .join("");
    result
}

pub async fn pasring_and_translate(text: String, from: &str, to: &str) -> Result<String, Error> {
    let text = text.replace("\r\n", "\\zzab").replace('\n', "\\zzab");

    let translated_text = translate(text, from, to).await?;
    let translated_text = translated_text.replace("\\zzab", "\n");
    Ok(translated_text)
}
