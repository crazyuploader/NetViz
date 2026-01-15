use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

const BASE_API_URL: &str = "https://www.peeringdb.com/api/";
const OUTPUT_DIR: &str = "data/peeringdb";

pub async fn fetch_and_save_peeringdb_data() -> Result<(), Box<dyn std::error::Error>> {
    let output_path = PathBuf::from(OUTPUT_DIR);
    fs::create_dir_all(&output_path)?;

    let client = reqwest::Client::builder()
        .user_agent("NetViz/0.1.0")
        .build()?;

    println!("Fetching API index from {}...", BASE_API_URL);
    let api_index: Value = client.get(BASE_API_URL).send().await?.json().await?;

    let endpoints = api_index["data"][0]
        .as_object()
        .ok_or("Invalid API index format")?;

    let api_key = std::env::var("PEERINGDB_API_KEY").unwrap_or_default();
    let mut headers = HeaderMap::new();
    if !api_key.is_empty() {
        println!("API Key for PeeringDB found, using it.");
        let auth_value = format!("Api-Key {}", api_key);
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth_value)?);
    }

    for (name, url) in endpoints {
        let url_str = url.as_str().ok_or("Invalid endpoint URL")?;
        let file_path = output_path.join(format!("{}.json", name));

        println!("Fetching data for '{}' from {}...", name, url_str);
        match client.get(url_str).headers(headers.clone()).send().await {
            Ok(resp) => {
                if let Ok(data) = resp.json::<Value>().await {
                    let json_data = serde_json::to_string_pretty(&data)?;
                    fs::write(&file_path, json_data)?;
                    println!("Successfully saved data to {:?}", file_path);
                }
            }
            Err(e) => eprintln!("Error fetching data from {}: {}", url_str, e),
        }
    }

    Ok(())
}
