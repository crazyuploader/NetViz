//! Data loading module - handles reading network data from JSON files.

use crate::models::{Network, PeeringDBResponse};
use std::fs;

/// Loads network data from the PeeringDB JSON file.
///
/// # Returns
/// - `Ok(Vec<Network>)` - A vector of network records if successful
/// - `Err(...)` - An error if the file can't be read or parsed
///
/// # Rust Concepts
/// - `Result<T, E>` is Rust's way of handling errors - it's either Ok(value) or Err(error)
/// - `Box<dyn std::error::Error>` is a "boxed trait object" - it can hold any error type
/// - The `?` operator below is shorthand for "return Err if this fails, otherwise unwrap Ok"
pub fn load_network_data() -> Result<Vec<Network>, Box<dyn std::error::Error>> {
    let file_path = "data/peeringdb/net.json";

    // `fs::read_to_string` reads entire file into a String
    // The `?` at the end propagates errors upward (returns early if error)
    let content = fs::read_to_string(file_path)?;

    // Parse JSON into our struct. `serde_json::from_str` deserializes the JSON.
    // The `::<PeeringDBResponse<Network>>` is a "turbofish" - tells Rust the target type
    let response: PeeringDBResponse<Network> = serde_json::from_str(&content)?;

    // Return the data field from the response
    // `Ok(...)` wraps the value in a successful Result
    Ok(response.data)
}
