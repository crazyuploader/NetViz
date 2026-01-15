//! Data loading module - handles reading network data from JSON files.

use crate::error::NetVizError;
use crate::models::{Network, PeeringDBResponse};
use std::fs;

/// Loads network data from the PeeringDB JSON file.
///
/// # Returns
/// - `Ok(Vec<Network>)` - A vector of network records if successful
/// - `Err(NetVizError)` - An error if the file can't be read or parsed
///
/// # Rust Concepts
/// - `Result<T, E>` is Rust's way of handling errors - it's either Ok(value) or Err(error)
/// - `NetVizError` is our custom error type that provides clear error messages
/// - The `?` operator below is shorthand for "return Err if this fails, otherwise unwrap Ok"
pub fn load_network_data() -> Result<Vec<Network>, NetVizError> {
    let file_path = "data/peeringdb/net.json";

    // `fs::read_to_string` reads entire file into a String
    // The `?` at the end propagates errors upward (returns early if error)
    // Thanks to `#[from]` in NetVizError, io::Error is automatically converted
    let content = fs::read_to_string(file_path)?;

    // Parse JSON into our struct. `serde_json::from_str` deserializes the JSON.
    // The `::<PeeringDBResponse<Network>>` is a "turbofish" - tells Rust the target type
    // JSON errors are also automatically converted to NetVizError::JsonParse
    let response: PeeringDBResponse<Network> = serde_json::from_str(&content)?;

    // Return the data field from the response
    // `Ok(...)` wraps the value in a successful Result
    Ok(response.data)
}
