use crate::models::{Network, PeeringDBResponse};
use std::fs;

/// Load network data from the PeeringDB JSON file.
/// Returns an error if the file cannot be read or parsed.
pub fn load_network_data() -> Result<Vec<Network>, Box<dyn std::error::Error>> {
    let file_path = "data/peeringdb/net.json";
    let content = fs::read_to_string(file_path)?;
    let response: PeeringDBResponse<Network> = serde_json::from_str(&content)?;
    Ok(response.data)
}
