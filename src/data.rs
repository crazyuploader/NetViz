//! Data loading module - handles reading network data from JSON files.

use crate::error::NetVizError;
use crate::models::{Network, PeeringDBResponse};
use std::fs;

/// Path to the network data file from PeeringDB.
const NETWORK_DATA_PATH: &str = "data/peeringdb/net.json";

/// Loads network data from the PeeringDB JSON file.
///
/// Reads and deserializes the cached PeeringDB network data from disk.
///
/// # Returns
///
/// * `Ok(Vec<Network>)` - Vector of network records if successful
/// * `Err(NetVizError)` - Error if file cannot be read or parsed
///
/// # Errors
///
/// Returns `NetVizError::Io` if the file cannot be read.
/// Returns `NetVizError::JsonParse` if the JSON is malformed.
pub fn load_network_data() -> Result<Vec<Network>, NetVizError> {
    let content = fs::read_to_string(NETWORK_DATA_PATH)?;
    let response: PeeringDBResponse<Network> = serde_json::from_str(&content)?;
    Ok(response.data)
}
