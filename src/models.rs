//! Data models - defines the structure of our data using Rust structs.
//!
//! These structs are the Rust equivalent of Python dataclasses or TypeScript interfaces.
//! They define what fields each data type has and their types.

use serde::{Deserialize, Serialize};

/// Generic wrapper for PeeringDB API responses.
///
/// The `<T>` makes this a "generic" struct - it can hold any type in its `data` field.
/// For example: `PeeringDBResponse<Network>` holds a `Vec<Network>`.
///
/// # Derive Macros
/// - `Debug` - Allows printing with `{:?}` for debugging
/// - `Serialize` - Allows converting to JSON (for API responses)
/// - `Deserialize` - Allows parsing from JSON (for API responses)
/// - `Clone` - Allows making copies of the struct
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PeeringDBResponse<T> {
    pub data: Vec<T>, // `pub` makes this field accessible from other modules
}

/// Represents a single network from PeeringDB.
///
/// # Field Types
/// - `i64` - A 64-bit signed integer (for large numbers like ASNs)
/// - `String` - An owned, growable string
/// - `Option<T>` - Either `Some(value)` or `None` (like nullable in other languages)
///
/// # Serde Behavior
/// When deserializing, missing JSON fields become `None` for Option types.
/// When serializing, `None` values are typically omitted from the output.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Network {
    pub id: i64,                        // PeeringDB internal ID
    pub name: String,                   // Network name (e.g., "Google")
    pub asn: i64,                       // Autonomous System Number
    pub aka: Option<String>,            // Also known as (alternative name)
    pub status: Option<String>,         // Network status (e.g., "ok", "deleted")
    pub info_type: Option<String>,      // Type: "NSP", "Content", "Cable/DSL/ISP", etc.
    pub policy_general: Option<String>, // Peering policy: "Open", "Selective", "Restrictive"
    pub info_scope: Option<String>,     // Geographic scope: "Global", "Regional", etc.
    pub info_prefixes4: Option<i64>,    // Number of IPv4 prefixes announced
    pub info_prefixes6: Option<i64>,    // Number of IPv6 prefixes announced
    pub ix_count: Option<i64>,          // Number of IXPs the network is present at
    pub fac_count: Option<i64>,         // Number of facilities the network is in
    pub website: Option<String>,        // Network's website URL
}

/// Statistics computed from the network data.
///
/// Uses `HashMap<String, usize>` to store counts:
/// - Key: The category name (e.g., "NSP", "Content")
/// - Value: How many networks belong to that category
///
/// # Why `usize`?
/// `usize` is the platform's native size type (32-bit on 32-bit systems, 64-bit on 64-bit).
/// It's commonly used for counts and indices because it matches pointer size.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Stats {
    pub total_networks: usize, // Total network count
    pub network_types: std::collections::HashMap<String, usize>, // Count by type
    pub policy_types: std::collections::HashMap<String, usize>, // Count by policy
    pub scopes: std::collections::HashMap<String, usize>, // Count by scope
}
