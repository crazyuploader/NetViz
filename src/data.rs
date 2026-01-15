use crate::models::{Network, PeeringDBResponse};
use std::fs;

pub fn load_network_data() -> Vec<Network> {
    let file_path = "data/peeringdb/net.json";

    match fs::read_to_string(file_path) {
        Ok(content) => match serde_json::from_str::<PeeringDBResponse<Network>>(&content) {
            Ok(response) => response.data,
            Err(e) => {
                eprintln!("An error occurred while decoding JSON: {}", e);
                Vec::new()
            }
        },
        Err(_) => {
            eprintln!("Error: The file '{}' was not found.", file_path);
            Vec::new()
        }
    }
}
