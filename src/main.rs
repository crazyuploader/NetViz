//! NetViz - Network Visualization Application
//!
//! Web application that displays network data from PeeringDB.
//! Features automatic data refresh via background scheduler.

mod data;
mod error;
mod fetcher;
mod models;

use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use itertools::Itertools;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tera::{Context, Tera};
use tokio::sync::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info, warn};

use crate::data::load_network_data;
use crate::fetcher::fetch_and_save_peeringdb_data;
use crate::models::{Network, Stats};

/// Path to the network data file from PeeringDB.
const NETWORK_DATA_PATH: &str = "data/peeringdb/net.json";

/// Default cron schedule: daily at midnight UTC.
const DEFAULT_REFRESH_CRON: &str = "0 0 0 * * *";

/// Application configuration from environment variables.
#[derive(Debug, Clone)]
struct Config {
    /// Address to bind the HTTP server to.
    bind_address: String,
    /// Cron expression for data refresh schedule.
    refresh_cron: String,
}

impl Config {
    /// Creates Config from environment variables with defaults.
    ///
    /// # Environment Variables
    ///
    /// * `BIND_ADDRESS` - Server bind address (default: "0.0.0.0:8201")
    /// * `REFRESH_CRON` - Cron expression for refresh schedule (default: daily at midnight)
    fn from_env() -> Self {
        Self {
            bind_address: std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8201".into()),
            refresh_cron: std::env::var("REFRESH_CRON")
                .unwrap_or_else(|_| DEFAULT_REFRESH_CRON.into()),
        }
    }
}

/// Shared application state passed to all request handlers.
///
/// Uses `RwLock` for the data field to allow concurrent reads from handlers
/// while permitting exclusive writes from the background refresh task.
struct AppState {
    /// Template engine for rendering HTML pages.
    tera: Tera,
    /// Network data protected by RwLock for thread-safe concurrent access.
    /// Multiple readers can access simultaneously; writer has exclusive access.
    data: RwLock<Vec<Network>>,
}

/// Query parameters for pagination.
#[derive(Deserialize)]
struct Pagination {
    page: Option<usize>,
    per_page: Option<usize>,
}

/// Query parameters for search.
#[derive(Deserialize)]
struct SearchQuery {
    asn: Option<i64>,
    name: Option<String>,
}

/// Renders a template with error handling.
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

/// Truncates a string to max_chars (UTF-8 safe), appending "..." if truncated.
fn truncate_chars(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count > max_chars {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}

/// Refreshes network data by fetching from PeeringDB and updating the shared state.
///
/// This function is called by the background scheduler on a cron schedule.
///
/// # Arguments
///
/// * `state` - Shared application state containing the RwLock-protected data
async fn refresh_data(state: Arc<AppState>) {
    info!("Starting scheduled data refresh from PeeringDB...");

    // Fetch new data from PeeringDB API
    if let Err(e) = fetch_and_save_peeringdb_data().await {
        error!("Failed to fetch data from PeeringDB: {}", e);
        return;
    }

    // Load the newly fetched data from disk
    match load_network_data() {
        Ok(new_data) => {
            let count = new_data.len();
            // Acquire write lock and update data
            // This blocks readers momentarily but ensures consistency
            let mut data_guard = state.data.write().await;
            *data_guard = new_data;
            info!("Data refresh complete: loaded {} networks", count);
        }
        Err(e) => {
            error!("Failed to load refreshed data: {}", e);
        }
    }
}

/// Creates and starts the background job scheduler for periodic data refresh.
///
/// # Arguments
///
/// * `cron_expr` - Cron expression defining the refresh schedule
/// * `state` - Shared application state to update on refresh
///
/// # Returns
///
/// * `Ok(JobScheduler)` - Running scheduler that must be kept alive
/// * `Err(...)` - If scheduler creation or job addition fails
async fn start_scheduler(
    cron_expr: &str,
    state: Arc<AppState>,
) -> Result<JobScheduler, Box<dyn std::error::Error + Send + Sync>> {
    let scheduler = JobScheduler::new().await?;

    // Clone state for the closure (moved into the job)
    let state_clone = state.clone();

    // Create a job that runs on the specified cron schedule
    let job = Job::new_async(cron_expr, move |_uuid, _lock| {
        let state = state_clone.clone();
        Box::pin(async move {
            refresh_data(state).await;
        })
    })?;

    scheduler.add(job).await?;
    scheduler.start().await?;

    info!("Background scheduler started with cron: {}", cron_expr);

    Ok(scheduler)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let config = Config::from_env();

    // Fetch data from PeeringDB if not cached locally
    if !std::path::Path::new(NETWORK_DATA_PATH).exists() {
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

    // Create shared state with RwLock for concurrent access
    let state = Arc::new(AppState {
        tera,
        data: RwLock::new(data),
    });

    // Start the background scheduler for periodic data refresh
    // Gracefully degrade if scheduler fails - app still works without auto-refresh
    let _scheduler: Option<JobScheduler> =
        match start_scheduler(&config.refresh_cron, state.clone()).await {
            Ok(s) => Some(s),
            Err(e) => {
                warn!(
                    "Failed to start background scheduler: {}. Continuing without auto-refresh.",
                    e
                );
                None
            }
        };

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
        .expect("TCP listener must bind to configured address for server to start");

    info!("Server listening on http://{}", config.bind_address);
    info!("Data refresh scheduled: {}", config.refresh_cron);

    if let Err(e) = axum::serve(listener, app).await {
        error!("Server error: {}", e);
        std::process::exit(1);
    }
}

/// GET / - Dashboard with network statistics.
async fn index(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Acquire read lock - multiple handlers can read concurrently
    let data = state.data.read().await;

    let mut network_types: HashMap<&str, usize> = HashMap::new();
    let mut policy_types: HashMap<&str, usize> = HashMap::new();
    let mut scopes: HashMap<&str, usize> = HashMap::new();

    for item in data.iter() {
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
        total_networks: data.len(),
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
    // Clone network data for template to allow dropping the read lock
    let networks: Vec<Network> = data.iter().take(10).cloned().collect();
    drop(data); // Release read lock before rendering
    context.insert("networks", &networks);

    render_template(&state.tera, "dashboard.html", &context)
}

/// GET /networks - Paginated network list.
async fn networks_list(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<Pagination>,
) -> impl IntoResponse {
    let data = state.data.read().await;
    let total_networks = data.len();

    if total_networks == 0 {
        drop(data);
        let mut context = Context::new();
        context.insert("networks", &Vec::<Network>::new());
        context.insert("page", &1usize);
        context.insert("per_page", &25usize);
        context.insert("total_pages", &0usize);
        context.insert("total_networks", &0usize);
        return render_template(&state.tera, "networks.html", &context);
    }

    let page = pagination.page.unwrap_or(1).max(1);
    let per_page = pagination.per_page.unwrap_or(25).clamp(1, 100);
    let total_pages = total_networks.div_ceil(per_page);
    let start_index = (page - 1).saturating_mul(per_page);
    let end_index = start_index.saturating_add(per_page).min(total_networks);

    // Clone paginated data before dropping lock
    let paginated_networks: Vec<Network> = if start_index >= total_networks {
        Vec::new()
    } else {
        data[start_index..end_index].to_vec()
    };
    drop(data);

    let mut context = Context::new();
    context.insert("networks", &paginated_networks);
    context.insert("page", &page);
    context.insert("per_page", &per_page);
    context.insert("total_pages", &total_pages);
    context.insert("total_networks", &total_networks);

    render_template(&state.tera, "networks.html", &context)
}

/// GET /analytics - Analytics dashboard.
async fn analytics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let context = Context::new();
    render_template(&state.tera, "analytics.html", &context)
}

/// GET /search - Search networks by ASN or name.
async fn search_networks(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let data = state.data.read().await;

    let search_name = query.name.as_ref().map(|n| {
        let mut s = n.clone();
        s.truncate(100);
        s.to_lowercase()
    });

    // Clone matching results before dropping lock
    let results: Vec<Network> = if query.asn.is_some() || search_name.is_some() {
        data.iter()
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
    drop(data);

    let mut context = Context::new();
    context.insert("results", &results);
    context.insert("query_asn", &query.asn);
    context.insert("query_name", &query.name);

    render_template(&state.tera, "search.html", &context)
}

/// GET /api/network-types - JSON network type counts.
async fn api_network_types(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let data = state.data.read().await;

    let mut network_types: HashMap<String, usize> = HashMap::new();
    for item in data.iter() {
        if let Some(ref t) = item.info_type {
            *network_types.entry(t.clone()).or_insert(0) += 1;
        }
    }
    drop(data);

    let (labels, counts): (Vec<String>, Vec<usize>) = network_types.into_iter().unzip();

    Json(serde_json::json!({
        "labels": labels,
        "data": counts
    }))
}

/// GET /api/prefixes-distribution - JSON prefix counts per network.
async fn api_prefixes_distribution(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let data = state.data.read().await;

    let results: Vec<_> = data
        .iter()
        .filter(|item| item.info_prefixes4.is_some() && item.info_prefixes6.is_some())
        .take(15)
        .map(|item| {
            let name = truncate_chars(&item.name, 30);
            (
                name,
                item.info_prefixes4.expect("filter guarantees Some"),
                item.info_prefixes6.expect("filter guarantees Some"),
            )
        })
        .collect();
    drop(data);

    let (networks, ipv4, ipv6): (Vec<_>, Vec<_>, Vec<_>) = results.into_iter().multiunzip();

    Json(serde_json::json!({
        "networks": networks,
        "ipv4": ipv4,
        "ipv6": ipv6
    }))
}

/// GET /api/ix-facility-correlation - JSON IX vs facility counts.
async fn api_ix_facility_correlation(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let data = state.data.read().await;

    let results: Vec<_> = data
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
    drop(data);

    Json(results)
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
        fn test_empty_string() {
            assert_eq!(truncate_chars("", 10), "");
        }

        #[test]
        fn test_unicode_characters() {
            assert_eq!(truncate_chars("こんにちは世界", 5), "こんにちは...");
        }

        #[test]
        fn test_emoji_characters() {
            assert_eq!(truncate_chars("Hello 🌍🌍🌍", 8), "Hello 🌍🌍...");
        }

        #[test]
        fn test_zero_max_chars() {
            assert_eq!(truncate_chars("Hello", 0), "...");
        }
    }

    mod pagination_tests {
        /// Helper to simulate pagination parameter processing.
        fn process_pagination(page: Option<usize>, per_page: Option<usize>) -> (usize, usize) {
            let page = page.unwrap_or(1).max(1);
            let per_page = per_page.unwrap_or(25).clamp(1, 100);
            (page, per_page)
        }

        #[test]
        fn test_page_defaults() {
            let (page, per_page) = process_pagination(None, None);
            assert_eq!(page, 1);
            assert_eq!(per_page, 25);
        }

        #[test]
        fn test_page_zero_becomes_one() {
            let (page, _) = process_pagination(Some(0), None);
            assert_eq!(page, 1);
        }

        #[test]
        fn test_per_page_clamped_to_max() {
            let (_, per_page) = process_pagination(None, Some(200));
            assert_eq!(per_page, 100);
        }

        #[test]
        fn test_per_page_clamped_to_min() {
            let (_, per_page) = process_pagination(None, Some(0));
            assert_eq!(per_page, 1);
        }

        #[test]
        fn test_total_pages_calculation() {
            let total_networks = 101_usize;
            let per_page = 25_usize;
            let total_pages = total_networks.div_ceil(per_page);
            assert_eq!(total_pages, 5);
        }

        #[test]
        fn test_slice_indices() {
            let page = 2_usize;
            let per_page = 25_usize;
            let total_networks = 100_usize;

            let start_index = (page - 1).saturating_mul(per_page);
            let end_index = start_index.saturating_add(per_page).min(total_networks);

            assert_eq!(start_index, 25);
            assert_eq!(end_index, 50);
        }

        #[test]
        fn test_last_page_partial() {
            let page = 5_usize;
            let per_page = 25_usize;
            let total_networks = 101_usize;

            let start_index = (page - 1).saturating_mul(per_page);
            let end_index = start_index.saturating_add(per_page).min(total_networks);

            assert_eq!(start_index, 100);
            assert_eq!(end_index, 101);
        }
    }
}
