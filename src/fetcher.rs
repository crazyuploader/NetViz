//! PeeringDB data fetcher - downloads network data from the PeeringDB API.

use crate::error::NetVizError;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{error, info, warn};

/// Base URL for the PeeringDB API.
const BASE_API_URL: &str = "https://www.peeringdb.com/api/";

/// Directory where downloaded JSON files are saved.
const OUTPUT_DIR: &str = "data/peeringdb";

/// Fetches all data from PeeringDB API and saves as JSON files.
///
/// Creates the output directory if it does not exist, discovers available
/// endpoints from the API index, and downloads each dataset. Individual
/// endpoint failures are logged but do not fail the overall operation.
///
/// # Returns
///
/// * `Ok(())` - All available endpoints were processed (some may have failed)
/// * `Err(NetVizError)` - Critical failure (cannot create directory, HTTP client, or API index)
///
/// # Errors
///
/// Returns `NetVizError::Io` if the output directory cannot be created.
/// Returns `NetVizError::HttpRequest` if the API index request fails.
/// Returns `NetVizError::InvalidApiResponse` if the API index format is unexpected.
pub async fn fetch_and_save_peeringdb_data() -> Result<(), NetVizError> {
    let output_path = PathBuf::from(OUTPUT_DIR);
    fs::create_dir_all(&output_path)?;

    let client = reqwest::Client::builder()
        .user_agent("NetViz/1.0.0")
        .timeout(Duration::from_secs(10))
        .build()?;

    info!("Fetching API index from {}...", BASE_API_URL);

    let api_index: Value = client.get(BASE_API_URL).send().await?.json().await?;

    let endpoints = api_index["data"][0]
        .as_object()
        .ok_or_else(|| NetVizError::InvalidApiResponse("Invalid API index format".to_string()))?;

    let api_key = std::env::var("PEERINGDB_API_KEY").unwrap_or_default();
    let mut headers = HeaderMap::new();
    if !api_key.is_empty() {
        info!("API Key for PeeringDB found, using it.");
        let auth_value = format!("Api-Key {}", api_key);
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth_value)?);
    }

    for (name, url) in endpoints {
        let url_str = match url.as_str() {
            Some(s) => s,
            None => {
                warn!("Invalid endpoint URL for '{}', skipping", name);
                continue;
            }
        };

        let file_path = output_path.join(format!("{}.json", name));
        info!("Fetching data for '{}' from {}...", name, url_str);

        match client.get(url_str).headers(headers.clone()).send().await {
            Ok(resp) => match resp.json::<Value>().await {
                Ok(data) => match serde_json::to_string_pretty(&data) {
                    Ok(json_data) => {
                        if let Err(e) = fs::write(&file_path, json_data) {
                            error!("Failed to write data to {:?}: {}", file_path, e);
                        } else {
                            info!("Successfully saved data to {:?}", file_path);
                        }
                    }
                    Err(e) => error!("Failed to serialize data for {}: {}", url_str, e),
                },
                Err(e) => error!("Failed to parse JSON from {}: {}", url_str, e),
            },
            Err(e) => error!("Error fetching data from {}: {}", url_str, e),
        }
    }

    Ok(())
}
