//! Data loading module - handles reading network data from JSON files.

use crate::error::NetVizError;
use crate::models::{Network, PeeringDBResponse};
use std::fs;

/// Loads network data from the PeeringDB JSON file.
///
/// # Returns
/// - `Ok(Vec<Network>)` - Network records if successful
/// - `Err(NetVizError)` - Error if file can't be read or parsed
pub fn load_network_data() -> Result<Vec<Network>, NetVizError> {
    let file_path = "data/peeringdb/net.json";
    let content = fs::read_to_string(file_path)?;
    let response: PeeringDBResponse<Network> = serde_json::from_str(&content)?;
    Ok(response.data)
}
