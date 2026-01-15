use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
};
use polars::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tera::Context;
use tracing::error;

use crate::models::{Network, Stats};
use crate::state::AppState;

/// Query parameters for network list matching and pagination.
#[derive(Debug, Deserialize)]
pub struct NetworkQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    #[serde(default, deserialize_with = "empty_string_as_none_str")]
    pub q: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none_str")]
    pub type_: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none_str")]
    pub policy: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none_str")]
    pub status: Option<String>,
}

/// Query parameters for search endpoint.
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    /// AS Number to search for.
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub asn: Option<i64>,
    /// Network name to search for.
    #[serde(default, deserialize_with = "empty_string_as_none_str")]
    pub name: Option<String>,
}

fn empty_string_as_none<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(s) if s.is_empty() => Ok(None),
        Some(s) => s.parse::<T>().map(Some).map_err(serde::de::Error::custom),
    }
}

fn empty_string_as_none_str<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(s) if s.is_empty() => Ok(None),
        Some(s) => Ok(Some(s)),
    }
}

fn render_template(
    tera: &tera::Tera,
    template: &str,
    context: &Context,
) -> Result<Html<String>, (StatusCode, &'static str)> {
    tera.render(template, context).map(Html).map_err(|e| {
        error!("Template render error for '{}': {}", template, e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Render error")
    })
}

fn truncate_chars(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count > max_chars {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}

/// GET / - Dashboard with network statistics.
pub async fn index(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let data_guard = state.data.read().await;
    let networks = &data_guard.networks;

    // Manual stats calculation on Vec is fast enough and easier than DF for simple counts
    // without aggregation queries, but since we have DF we could use it.
    // However, existing logic works fine for simple stats map.
    // Let's keep existing logic to minimize risk, but use DF if I wanted to be "pure".
    // Actually, stick to logic from previous main.rs for dashboard stats to ensure correctness.

    let mut network_types: HashMap<&str, usize> = HashMap::new();
    let mut policy_types: HashMap<&str, usize> = HashMap::new();
    let mut scopes: HashMap<&str, usize> = HashMap::new();

    for item in networks.iter() {
        if let Some(ref t) = item.info_type {
            *network_types.entry(t.as_str()).or_insert(0) += 1;
        }
        if let Some(ref p) = item.policy_general {
            *policy_types.entry(p.as_str()).or_insert(0) += 1;
        }
        if let Some(ref s) = item.info_scope {
            *scopes.entry(s.as_str()).or_insert(0) += 1;
        }
    }

    let stats = Stats {
        total_networks: networks.len(),
        network_types: network_types
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
        policy_types: policy_types
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
        scopes: scopes
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    };

    let mut context = Context::new();
    context.insert("stats", &stats);
    let recent_networks: Vec<Network> = networks.iter().take(10).cloned().collect();
    drop(data_guard);
    context.insert("networks", &recent_networks);

    render_template(&state.tera, "dashboard.html", &context)
}

/// GET /networks - Paginated network list.
pub async fn networks_list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<NetworkQuery>,
) -> impl IntoResponse {
    let data_guard = state.data.read().await;
    let networks = &data_guard.networks;

    // Filter first
    let filtered_networks: Vec<Network> = networks
        .iter()
        .filter(|n| {
            let mut matches = true;

            if let Some(ref q) = query.q {
                let q_lower = q.to_lowercase();
                matches &= n.name.to_lowercase().contains(&q_lower)
                    || n.asn.to_string().contains(&q_lower)
                    || n.aka
                        .as_ref()
                        .map_or(false, |a| a.to_lowercase().contains(&q_lower));
            }

            if let Some(ref t) = query.type_ {
                matches &= n
                    .info_type
                    .as_ref()
                    .map_or(false, |it| it.eq_ignore_ascii_case(t));
            }

            if let Some(ref p) = query.policy {
                matches &= n
                    .policy_general
                    .as_ref()
                    .map_or(false, |pg| pg.eq_ignore_ascii_case(p));
            }

            if let Some(ref s) = query.status {
                matches &= n
                    .status
                    .as_ref()
                    .map_or(false, |st| st.eq_ignore_ascii_case(s));
            }

            matches
        })
        .cloned()
        .collect();

    let total_networks = filtered_networks.len();
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(25).clamp(1, 100);
    let total_pages = total_networks.div_ceil(per_page);

    // Adjust page if it exceeds total pages (unless total is 0)
    let page = if total_pages > 0 && page > total_pages {
        total_pages
    } else {
        page
    };

    let start_index = (page - 1).saturating_mul(per_page);
    let end_index = start_index.saturating_add(per_page).min(total_networks);

    let paginated_networks: Vec<Network> = if start_index >= total_networks {
        Vec::new()
    } else {
        filtered_networks[start_index..end_index].to_vec()
    };
    drop(data_guard);

    let mut context = Context::new();
    context.insert("networks", &paginated_networks);
    context.insert("page", &page);
    context.insert("per_page", &per_page);
    context.insert("total_pages", &total_pages);
    context.insert("total_networks", &total_networks);

    // Pass back filter params
    context.insert("q", &query.q);
    context.insert("type_filter", &query.type_);
    context.insert("policy_filter", &query.policy);
    context.insert("status_filter", &query.status);

    render_template(&state.tera, "networks.html", &context)
}

/// GET /analytics - Analytics dashboard.
pub async fn analytics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let context = Context::new();
    render_template(&state.tera, "analytics.html", &context)
}

/// GET /search - Search networks.
pub async fn search_networks(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let data_guard = state.data.read().await;
    let networks = &data_guard.networks;

    let search_name = query.name.as_ref().map(|n| {
        let mut s = n.clone();
        s.truncate(100);
        s.to_lowercase()
    });

    let results: Vec<Network> = if query.asn.is_some() || search_name.is_some() {
        networks
            .iter()
            .filter(|network| {
                let matches_asn = query.asn == Some(network.asn);
                let matches_name = search_name
                    .as_ref()
                    .is_some_and(|name| network.name.to_lowercase().contains(name));
                matches_asn || matches_name
            })
            .cloned()
            .collect()
    } else {
        Vec::new()
    };
    drop(data_guard);

    let mut context = Context::new();
    context.insert("results", &results);
    context.insert("query_asn", &query.asn);
    context.insert("query_name", &query.name);

    render_template(&state.tera, "search.html", &context)
}

/// GET /api/network-types - JSON network type counts using Polars.
pub async fn api_network_types(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let data_guard = state.data.read().await;
    let df = &data_guard.df;

    // Use Polars Lazy API for efficient aggregation
    let agg_result = df
        .clone()
        .lazy()
        .filter(col("info_type").is_not_null())
        .group_by([col("info_type")])
        .agg([len().alias("count")])
        .collect();

    drop(data_guard);

    match agg_result {
        Ok(res) => {
            // Extract columns
            let labels: Vec<String> = res
                .column("info_type")
                .ok()
                .and_then(|s| s.str().ok())
                .map(|ca| ca.into_iter().flatten().map(|s| s.to_string()).collect())
                .unwrap_or_default();

            let counts: Vec<usize> = if let Ok(s) = res.column("count") {
                if let Ok(cast_s) = s.cast(&DataType::UInt64) {
                    if let Ok(ca) = cast_s.u64() {
                        ca.into_iter().flatten().map(|v| v as usize).collect()
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            } else {
                vec![]
            };

            Json(serde_json::json!({
                "labels": labels,
                "data": counts
            }))
        }
        Err(e) => {
            error!("Polars aggregation error: {}", e);
            Json(serde_json::json!({"labels": [], "data": []}))
        }
    }
}

/// GET /api/prefixes-distribution - Top 15 networks by prefixes using Polars.
pub async fn api_prefixes_distribution(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let data_guard = state.data.read().await;
    let df = &data_guard.df;

    let result = df
        .clone()
        .lazy()
        .filter(
            col("info_prefixes4")
                .is_not_null()
                .and(col("info_prefixes6").is_not_null()),
        )
        .select([col("name"), col("info_prefixes4"), col("info_prefixes6")])
        .limit(15) // Just take first 15 as in original code, or sort? Original used iter().take(15)
        .collect();

    drop(data_guard);

    match result {
        Ok(res) => {
            let names: Vec<String> = res
                .column("name")
                .ok()
                .and_then(|s| s.str().ok())
                .map(|ca| {
                    ca.into_iter()
                        .flatten()
                        .map(|s| truncate_chars(s, 30))
                        .collect()
                })
                .unwrap_or_default();

            let ipv4: Vec<i64> = res
                .column("info_prefixes4")
                .ok()
                .and_then(|s| s.i64().ok())
                .map(|ca| ca.into_iter().flatten().collect())
                .unwrap_or_default();

            let ipv6: Vec<i64> = res
                .column("info_prefixes6")
                .ok()
                .and_then(|s| s.i64().ok())
                .map(|ca| ca.into_iter().flatten().collect())
                .unwrap_or_default();

            Json(serde_json::json!({
                "networks": names,
                "ipv4": ipv4,
                "ipv6": ipv6
            }))
        }
        Err(e) => {
            error!("Polars error: {}", e);
            Json(serde_json::json!({"networks": [], "ipv4": [], "ipv6": []}))
        }
    }
}

/// GET /api/ix-facility-correlation - Scatter plot data using Polars.
pub async fn api_ix_facility_correlation(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let data_guard = state.data.read().await;
    let df = &data_guard.df;

    let result = df
        .clone()
        .lazy()
        .filter(
            col("ix_count")
                .is_not_null()
                .and(col("fac_count").is_not_null()),
        )
        .select([col("ix_count"), col("fac_count"), col("name")])
        .collect();

    drop(data_guard);

    match result {
        Ok(res) => {
            let ix: Vec<i64> = res
                .column("ix_count")
                .unwrap()
                .i64()
                .unwrap()
                .into_iter()
                .flatten()
                .collect();
            let fac: Vec<i64> = res
                .column("fac_count")
                .unwrap()
                .i64()
                .unwrap()
                .into_iter()
                .flatten()
                .collect();
            let name_ca = res.column("name").unwrap().str().unwrap();

            let points: Vec<_> = ix
                .iter()
                .zip(fac.iter())
                .zip(name_ca.into_iter())
                .filter_map(|((x, y), n)| {
                    n.map(|name| {
                        serde_json::json!({
                            "x": x,
                            "y": y,
                            "label": name
                        })
                    })
                })
                .collect();

            Json(points)
        }
        Err(e) => {
            error!("Polars error: {}", e);
            Json(Vec::<serde_json::Value>::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod truncate_chars_tests {
        use super::*;

        #[test]
        fn test_short_string_unchanged() {
            assert_eq!(truncate_chars("Hello", 10), "Hello");
        }

        #[test]
        fn test_exact_length_unchanged() {
            assert_eq!(truncate_chars("Hello", 5), "Hello");
        }

        #[test]
        fn test_long_string_truncated() {
            assert_eq!(truncate_chars("Hello, World!", 5), "Hello...");
        }

        #[test]
        fn test_emoji_characters() {
            assert_eq!(truncate_chars("Hello 🌍🌍🌍", 8), "Hello 🌍🌍...");
        }
    }
}
