//! PeeringDB data fetcher - downloads network data from the PeeringDB API.

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use tracing::{error, info, warn};

/// Base URL for the PeeringDB API
const BASE_API_URL: &str = "https://www.peeringdb.com/api/";

/// Directory where we save downloaded JSON files
const OUTPUT_DIR: &str = "data/peeringdb";

/// Fetches all data from PeeringDB API and saves it as JSON files.
///
/// # How it works
/// 1. Creates the output directory if it doesn't exist
/// 2. Fetches the API index to discover all available endpoints
/// 3. Iterates through each endpoint and downloads its data
/// 4. Saves each dataset as a separate JSON file
///
/// # Rust Concepts
/// - `async fn` - This function can be paused while waiting for I/O (like HTTP requests)
/// - `.await` - Pauses execution until the async operation completes
/// - `Result<(), ...>` - Returns Ok(()) on success (unit type), or an error
pub async fn fetch_and_save_peeringdb_data() -> Result<(), Box<dyn std::error::Error>> {
    // Create output directory (and parent directories if needed)
    let output_path = PathBuf::from(OUTPUT_DIR);
    fs::create_dir_all(&output_path)?;

    // Build HTTP client with custom User-Agent (some APIs require this)
    let client = reqwest::Client::builder()
        .user_agent("NetViz/0.1.0")
        .build()?;

    info!("Fetching API index from {}...", BASE_API_URL);

    // `Value` is a generic JSON type - we use it when we don't know the exact structure
    // `.send().await` makes the HTTP request
    // `.json().await` parses the response body as JSON
    let api_index: Value = client.get(BASE_API_URL).send().await?.json().await?;

    // Navigate the JSON structure: data[0] contains the endpoint map
    // `.as_object()` tries to interpret it as a JSON object (returns Option)
    // `.ok_or(...)` converts None to an error
    let endpoints = api_index["data"][0]
        .as_object()
        .ok_or("Invalid API index format")?;

    // Check for API key in environment variable (optional but recommended)
    let api_key = std::env::var("PEERINGDB_API_KEY").unwrap_or_default();
    let mut headers = HeaderMap::new();
    if !api_key.is_empty() {
        info!("API Key for PeeringDB found, using it.");
        let auth_value = format!("Api-Key {}", api_key);
        // `HeaderValue::from_str()` can fail if the string contains invalid chars
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth_value)?);
    }

    // Iterate over all API endpoints
    for (name, url) in endpoints {
        // Handle invalid URLs gracefully - skip instead of aborting
        let url_str = match url.as_str() {
            Some(s) => s,
            None => {
                warn!("Invalid endpoint URL for '{}', skipping", name);
                continue;
            }
        };

        let file_path = output_path.join(format!("{}.json", name));

        info!("Fetching data for '{}' from {}...", name, url_str);

        // `match` handles both success and error cases
        // `.headers(headers.clone())` attaches our auth headers
        match client.get(url_str).headers(headers.clone()).send().await {
            Ok(resp) => {
                // Handle JSON parse failures explicitly instead of silently ignoring
                match resp.json::<Value>().await {
                    Ok(data) => {
                        // Pretty-print JSON with indentation
                        let json_data = serde_json::to_string_pretty(&data)?;
                        // Write to file
                        fs::write(&file_path, json_data)?;
                        info!("Successfully saved data to {:?}", file_path);
                    }
                    Err(e) => {
                        error!("Failed to parse JSON from {}: {}", url_str, e);
                        // Continue to next endpoint instead of failing
                    }
                }
            }
            Err(e) => error!("Error fetching data from {}: {}", url_str, e),
        }
    }

    Ok(())
}
