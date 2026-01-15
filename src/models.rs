//! Data models for network data structures.

use serde::{Deserialize, Serialize};

/// Generic wrapper for PeeringDB API responses.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PeeringDBResponse<T> {
    pub data: Vec<T>,
}

/// Represents a single network from PeeringDB.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Network {
    pub id: i64,
    pub name: String,
    pub asn: i64,
    pub aka: Option<String>,
    pub status: Option<String>,
    pub info_type: Option<String>,
    pub policy_general: Option<String>,
    pub info_scope: Option<String>,
    pub info_prefixes4: Option<i64>,
    pub info_prefixes6: Option<i64>,
    pub ix_count: Option<i64>,
    pub fac_count: Option<i64>,
    pub website: Option<String>,
}

/// Statistics computed from network data.
///
/// Aggregates counts of networks by type, policy, and geographic scope.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Stats {
    /// Total number of networks in the dataset.
    pub total_networks: usize,
    /// Count of networks grouped by info_type (e.g., "NSP", "Cable/DSL/ISP").
    pub network_types: std::collections::HashMap<String, usize>,
    /// Count of networks grouped by peering policy (e.g., "Open", "Selective").
    pub policy_types: std::collections::HashMap<String, usize>,
    /// Count of networks grouped by geographic scope (e.g., "Regional", "Global").
    pub scopes: std::collections::HashMap<String, usize>,
}
