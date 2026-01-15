mod data;
mod fetcher;
mod models;

use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tera::{Context, Tera};
use tracing::{error, info};

use crate::data::load_network_data;
use crate::fetcher::fetch_and_save_peeringdb_data;
use crate::models::{Network, Stats};

/// Application configuration loaded from environment variables.
#[derive(Debug)]
struct Config {
    bind_address: String,
}

impl Config {
    fn from_env() -> Self {
        Self {
            bind_address: std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8201".into()),
        }
    }
}

/// Shared application state.
struct AppState {
    tera: Tera,
    data: Vec<Network>,
}

#[derive(Deserialize)]
struct Pagination {
    page: Option<usize>,
    per_page: Option<usize>,
}

#[derive(Deserialize)]
struct SearchQuery {
    asn: Option<i64>,
    name: Option<String>,
}

/// Helper function to render templates with consistent error handling.
fn render_template(
    tera: &Tera,
    template: &str,
    context: &Context,
) -> Result<Html<String>, (axum::http::StatusCode, &'static str)> {
    tera.render(template, context).map(Html).map_err(|e| {
        error!("Template render error for '{}': {}", template, e);
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Render error",
        )
    })
}

#[tokio::main]
async fn main() {
    // Initialize structured logging
    tracing_subscriber::fmt::init();

    let config = Config::from_env();

    // Check if we need to fetch data first
    if !std::path::Path::new("data/peeringdb/net.json").exists() {
        info!("Fetching initial data from PeeringDB...");
        if let Err(e) = fetch_and_save_peeringdb_data().await {
            error!("Failed to fetch initial data: {}", e);
        }
    }

    let data = match load_network_data() {
        Ok(d) => {
            info!("Loaded {} networks from data file", d.len());
            d
        }
        Err(e) => {
            error!("Failed to load network data: {}", e);
            std::process::exit(1);
        }
    };

    let tera = match Tera::new("templates/**/*.html") {
        Ok(t) => t,
        Err(e) => {
            error!("Template parsing error(s): {}", e);
            std::process::exit(1);
        }
    };

    let state = Arc::new(AppState { tera, data });

    let app = Router::new()
        .route("/", get(index))
        .route("/networks", get(networks_list))
        .route("/analytics", get(analytics))
        .route("/search", get(search_networks))
        .route("/api/network-types", get(api_network_types))
        .route("/api/prefixes-distribution", get(api_prefixes_distribution))
        .route(
            "/api/ix-facility-correlation",
            get(api_ix_facility_correlation),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&config.bind_address)
        .await
        .unwrap_or_else(|e| {
            error!("Failed to bind to {}: {}", config.bind_address, e);
            std::process::exit(1);
        });

    info!("Server listening on http://{}", config.bind_address);

    if let Err(e) = axum::serve(listener, app).await {
        error!("Server error: {}", e);
        std::process::exit(1);
    }
}

async fn index(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut network_types: HashMap<&str, usize> = HashMap::new();
    let mut policy_types: HashMap<&str, usize> = HashMap::new();
    let mut scopes: HashMap<&str, usize> = HashMap::new();

    for item in &state.data {
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

    // Convert to owned for serialization
    let stats = Stats {
        total_networks: state.data.len(),
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

    let networks: Vec<&Network> = state.data.iter().take(10).collect();
    context.insert("networks", &networks);

    render_template(&state.tera, "dashboard.html", &context)
}

async fn networks_list(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<Pagination>,
) -> impl IntoResponse {
    // Validate and clamp pagination values
    let page = pagination.page.unwrap_or(1).max(1);
    let per_page = pagination.per_page.unwrap_or(25).clamp(1, 100);

    let total_networks = state.data.len();
    let total_pages = (total_networks + per_page - 1) / per_page;

    let start_index = (page - 1) * per_page;
    let end_index = (start_index + per_page).min(total_networks);

    // Handle out-of-bounds page
    let paginated_networks: Vec<&Network> = if start_index < total_networks {
        state.data[start_index..end_index].iter().collect()
    } else {
        Vec::new()
    };

    let mut context = Context::new();
    context.insert("networks", &paginated_networks);
    context.insert("page", &page);
    context.insert("per_page", &per_page);
    context.insert("total_pages", &total_pages);
    context.insert("total_networks", &total_networks);

    render_template(&state.tera, "networks.html", &context)
}

async fn analytics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let context = Context::new();
    render_template(&state.tera, "analytics.html", &context)
}

async fn search_networks(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    // Truncate search query for safety
    let search_name = query.name.as_ref().map(|n| {
        let mut s = n.clone();
        s.truncate(100);
        s.to_lowercase()
    });

    let results: Vec<&Network> = if query.asn.is_some() || search_name.is_some() {
        state
            .data
            .iter()
            .filter(|network| {
                let matches_asn = query.asn.map_or(false, |asn| network.asn == asn);
                let matches_name = search_name
                    .as_ref()
                    .map_or(false, |name| network.name.to_lowercase().contains(name));
                matches_asn || matches_name
            })
            .collect()
    } else {
        Vec::new()
    };

    let mut context = Context::new();
    context.insert("results", &results);
    context.insert("query_asn", &query.asn);
    context.insert("query_name", &query.name);

    render_template(&state.tera, "search.html", &context)
}

async fn api_network_types(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut network_types: HashMap<&str, usize> = HashMap::new();
    for item in &state.data {
        if let Some(ref t) = item.info_type {
            *network_types.entry(t.as_str()).or_insert(0) += 1;
        }
    }

    let labels: Vec<&str> = network_types.keys().copied().collect();
    let data: Vec<usize> = network_types.values().copied().collect();

    Json(serde_json::json!({
        "labels": labels,
        "data": data
    }))
}

async fn api_prefixes_distribution(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let data: Vec<_> = state
        .data
        .iter()
        .filter(|item| item.info_prefixes4.is_some() && item.info_prefixes6.is_some())
        .take(15)
        .map(|item| {
            let name = if item.name.len() > 30 {
                format!("{}...", &item.name[..30])
            } else {
                item.name.clone()
            };
            (
                name,
                item.info_prefixes4.unwrap(),
                item.info_prefixes6.unwrap(),
            )
        })
        .collect();

    let (networks, ipv4, ipv6): (Vec<_>, Vec<_>, Vec<_>) = data.into_iter().fold(
        (vec![], vec![], vec![]),
        |(mut ns, mut v4s, mut v6s), (n, v4, v6)| {
            ns.push(n);
            v4s.push(v4);
            v6s.push(v6);
            (ns, v4s, v6s)
        },
    );

    Json(serde_json::json!({
        "networks": networks,
        "ipv4": ipv4,
        "ipv6": ipv6
    }))
}

async fn api_ix_facility_correlation(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let data: Vec<_> = state
        .data
        .iter()
        .filter_map(|item| match (item.ix_count, item.fac_count) {
            (Some(ix), Some(fac)) => Some(serde_json::json!({
                "x": ix,
                "y": fac,
                "label": &item.name
            })),
            _ => None,
        })
        .collect();

    Json(data)
}
