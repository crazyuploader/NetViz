use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PeeringDBResponse<T> {
    pub data: Vec<T>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Network {
    pub id: i64,
    pub name: String,
    pub asn: i64,
    pub info_type: Option<String>,
    pub policy_general: Option<String>,
    pub info_scope: Option<String>,
    pub info_prefixes4: Option<i64>,
    pub info_prefixes6: Option<i64>,
    pub ix_count: Option<i64>,
    pub fac_count: Option<i64>,
    pub website: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Stats {
    pub total_networks: usize,
    pub network_types: std::collections::HashMap<String, usize>,
    pub policy_types: std::collections::HashMap<String, usize>,
    pub scopes: std::collections::HashMap<String, usize>,
}
