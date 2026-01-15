//! NetViz - Network Visualization Application
//!
//! A web application that displays network data from PeeringDB.
//! Built with Axum (web framework), Tera (templates), and Reqwest (HTTP client).

// Import our local modules (other .rs files in src/)
mod data; // Handles loading data from JSON files
mod fetcher; // Handles fetching data from PeeringDB API
mod models; // Defines data structures (structs)

// --- External crate imports ---
// Axum is our web framework (like Flask/Express)
use axum::{
    extract::{Query, State}, // Extract query params and app state from requests
    response::{Html, IntoResponse, Json}, // Response types we can return
    routing::get,            // HTTP GET route helper
    Router,                  // The main router that maps URLs to handlers
};
use serde::Deserialize; // Allows parsing JSON/query strings into structs
use std::collections::HashMap; // Key-value dictionary type
use std::sync::Arc; // Thread-safe reference counting pointer (for shared state)
use tera::{Context, Tera}; // Template engine (like Jinja2)
use tracing::{error, info}; // Structured logging macros

// Import from our local modules
use crate::data::load_network_data;
use crate::fetcher::fetch_and_save_peeringdb_data;
use crate::models::{Network, Stats};

/// Application configuration loaded from environment variables.
/// In Rust, we use structs to group related data together.
#[derive(Debug)] // Allows printing this struct for debugging
struct Config {
    bind_address: String, // The address:port the server listens on
}

impl Config {
    /// Creates a Config by reading from environment variables.
    /// `unwrap_or_else` provides a default value if the env var isn't set.
    fn from_env() -> Self {
        Self {
            bind_address: std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8201".into()),
        }
    }
}

/// Shared application state - this is passed to all request handlers.
/// Arc<AppState> allows multiple threads to access this safely.
struct AppState {
    tera: Tera,         // Template engine instance
    data: Vec<Network>, // All network data loaded from JSON
}

/// Query parameters for pagination (e.g., /networks?page=2&per_page=50)
/// Serde's Deserialize trait automatically parses URL query strings.
#[derive(Deserialize)]
struct Pagination {
    page: Option<usize>, // Option means it can be None (not provided)
    per_page: Option<usize>,
}

/// Query parameters for search (e.g., /search?asn=64512&name=Google)
#[derive(Deserialize)]
struct SearchQuery {
    asn: Option<i64>,     // ASN to search for
    name: Option<String>, // Network name to search for
}

/// Helper function to render templates with consistent error handling.
/// Returns either HTML content or an error status code.
fn render_template(
    tera: &Tera,
    template: &str,
    context: &Context,
) -> Result<Html<String>, (axum::http::StatusCode, &'static str)> {
    // `map` transforms Ok(rendered_string) into Ok(Html(rendered_string))
    // `map_err` transforms Err(tera_error) into our error tuple
    tera.render(template, context).map(Html).map_err(|e| {
        error!("Template render error for '{}': {}", template, e);
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Render error",
        )
    })
}

/// Truncates a string to the specified number of characters (UTF-8 safe).
/// Appends "..." if the string was truncated.
fn truncate_chars(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count > max_chars {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}

/// The main entry point. `#[tokio::main]` sets up the async runtime.
/// `async fn` means this function can pause and resume (for I/O operations).
#[tokio::main]
async fn main() {
    // Initialize the logging system - logs will appear in terminal
    tracing_subscriber::fmt::init();

    let config = Config::from_env();

    // Check if data file exists; if not, fetch it from PeeringDB
    // `!` is the "not" operator
    if !std::path::Path::new("data/peeringdb/net.json").exists() {
        info!("Fetching initial data from PeeringDB...");
        // `if let Err(e)` executes the block only if there's an error
        if let Err(e) = fetch_and_save_peeringdb_data().await {
            error!("Failed to fetch initial data: {}", e);
        }
    }

    // Load network data from JSON file
    // `match` is like a switch statement but more powerful
    let data = match load_network_data() {
        Ok(d) => {
            info!("Loaded {} networks from data file", d.len());
            d // Return the data (no semicolon = return value)
        }
        Err(e) => {
            error!("Failed to load network data: {}", e);
            std::process::exit(1); // Exit with error code
        }
    };

    // Initialize template engine - loads all .html files from templates/
    let tera = match Tera::new("templates/**/*.html") {
        Ok(t) => t,
        Err(e) => {
            error!("Template parsing error(s): {}", e);
            std::process::exit(1);
        }
    };

    // Wrap state in Arc for thread-safe sharing between request handlers
    let state = Arc::new(AppState { tera, data });

    // Define routes - each .route() maps a URL path to a handler function
    let app = Router::new()
        .route("/", get(index)) // Dashboard
        .route("/networks", get(networks_list)) // Paginated list
        .route("/analytics", get(analytics)) // Analytics page
        .route("/search", get(search_networks)) // Search page
        .route("/api/network-types", get(api_network_types))
        .route("/api/prefixes-distribution", get(api_prefixes_distribution))
        .route(
            "/api/ix-facility-correlation",
            get(api_ix_facility_correlation),
        )
        .with_state(state); // Attach shared state to all routes

    // Bind to address and start listening
    // `await` pauses until the async operation completes
    let listener = tokio::net::TcpListener::bind(&config.bind_address)
        .await
        .unwrap_or_else(|e| {
            error!("Failed to bind to {}: {}", config.bind_address, e);
            std::process::exit(1);
        });

    info!("Server listening on http://{}", config.bind_address);

    // Start the server - runs forever until killed
    if let Err(e) = axum::serve(listener, app).await {
        error!("Server error: {}", e);
        std::process::exit(1);
    }
}

/// Handler for GET / - displays the dashboard with statistics.
/// `State(state)` extracts our AppState from the request.
/// `impl IntoResponse` means "returns something that can become an HTTP response".
async fn index(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Count occurrences of each network type, policy, and scope
    // Using &str (string slice) instead of String avoids cloning
    let mut network_types: HashMap<&str, usize> = HashMap::new();
    let mut policy_types: HashMap<&str, usize> = HashMap::new();
    let mut scopes: HashMap<&str, usize> = HashMap::new();

    // Iterate over all networks and count categories
    for item in &state.data {
        // `if let Some(ref t)` checks if the Option has a value and borrows it
        if let Some(ref t) = item.info_type {
            // `entry().or_insert(0)` gets or creates the entry, then `+= 1` increments
            *network_types.entry(t.as_str()).or_insert(0) += 1;
        }
        if let Some(ref p) = item.policy_general {
            *policy_types.entry(p.as_str()).or_insert(0) += 1;
        }
        if let Some(ref s) = item.info_scope {
            *scopes.entry(s.as_str()).or_insert(0) += 1;
        }
    }

    // Convert to owned Strings for serialization (templates need owned data)
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

    // Build template context (variables available in template)
    let mut context = Context::new();
    context.insert("stats", &stats);

    // Take first 10 networks for "recent networks" display
    let networks: Vec<&Network> = state.data.iter().take(10).collect();
    context.insert("networks", &networks);

    render_template(&state.tera, "dashboard.html", &context)
}

/// Handler for GET /networks - displays paginated network list.
/// `Query(pagination)` extracts query parameters from the URL.
async fn networks_list(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<Pagination>,
) -> impl IntoResponse {
    let total_networks = state.data.len();

    // Handle empty data case
    if total_networks == 0 {
        let mut context = Context::new();
        context.insert("networks", &Vec::<&Network>::new());
        context.insert("page", &1usize);
        context.insert("per_page", &25usize);
        context.insert("total_pages", &0usize);
        context.insert("total_networks", &0usize);
        return render_template(&state.tera, "networks.html", &context);
    }

    // Get page/per_page with defaults, and validate bounds
    // Treat 0 as 1, ensure at least 1
    let page = pagination.page.unwrap_or(1).max(1);
    let per_page = pagination.per_page.unwrap_or(25).clamp(1, 100);

    // Integer division with ceiling: (a + b - 1) / b
    let total_pages = (total_networks + per_page - 1) / per_page;

    // Calculate slice indices safely using saturating arithmetic
    let start_index = (page - 1).saturating_mul(per_page);
    let end_index = start_index.saturating_add(per_page).min(total_networks);

    // Handle out-of-bounds page numbers gracefully
    let paginated_networks: Vec<&Network> = if start_index >= total_networks {
        Vec::new()
    } else {
        state.data[start_index..end_index].iter().collect()
    };

    let mut context = Context::new();
    context.insert("networks", &paginated_networks);
    context.insert("page", &page);
    context.insert("per_page", &per_page);
    context.insert("total_pages", &total_pages);
    context.insert("total_networks", &total_networks);

    render_template(&state.tera, "networks.html", &context)
}

/// Handler for GET /analytics - displays analytics dashboard.
async fn analytics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let context = Context::new();
    render_template(&state.tera, "analytics.html", &context)
}

/// Handler for GET /search - searches networks by ASN or name.
async fn search_networks(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    // Truncate search query to prevent abuse (max 100 chars)
    let search_name = query.name.as_ref().map(|n| {
        let mut s = n.clone();
        s.truncate(100);
        s.to_lowercase()
    });

    // Only search if at least one search parameter is provided
    let results: Vec<&Network> = if query.asn.is_some() || search_name.is_some() {
        state
            .data
            .iter()
            // `.filter()` keeps only items where the closure returns true
            .filter(|network| {
                // `map_or(false, ...)` returns false if None, otherwise evaluates closure
                let matches_asn = query.asn.map_or(false, |asn| network.asn == asn);
                let matches_name = search_name
                    .as_ref()
                    .map_or(false, |name| network.name.to_lowercase().contains(name));
                matches_asn || matches_name // Match if either condition is true
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

/// API endpoint: GET /api/network-types - returns JSON with network type counts.
async fn api_network_types(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Collect entries as pairs to preserve label-data correspondence
    let entries: Vec<(&str, usize)> = {
        let mut network_types: HashMap<&str, usize> = HashMap::new();
        for item in &state.data {
            if let Some(ref t) = item.info_type {
                *network_types.entry(t.as_str()).or_insert(0) += 1;
            }
        }
        network_types.into_iter().collect()
    };

    // Split into aligned vectors
    let (labels, data): (Vec<&str>, Vec<usize>) = entries.into_iter().unzip();

    // `Json()` automatically serializes to JSON and sets Content-Type header
    Json(serde_json::json!({
        "labels": labels,
        "data": data
    }))
}

/// API endpoint: GET /api/prefixes-distribution - returns prefix counts per network.
async fn api_prefixes_distribution(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Chain multiple iterator operations for cleaner code
    let data: Vec<_> = state
        .data
        .iter()
        .filter(|item| item.info_prefixes4.is_some() && item.info_prefixes6.is_some())
        .take(15) // Limit to 15 for chart readability
        .map(|item| {
            // Use UTF-8 safe truncation
            let name = truncate_chars(&item.name, 30);
            (
                name,
                item.info_prefixes4.unwrap(),
                item.info_prefixes6.unwrap(),
            )
        })
        .collect();

    // Unzip tuple vector into three separate vectors
    // `fold` accumulates values - starting with empty vectors, push each item
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

/// API endpoint: GET /api/ix-facility-correlation - returns IX vs facility counts.
async fn api_ix_facility_correlation(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // `filter_map` combines filter and map - returns None to skip, Some(x) to include x
    let data: Vec<_> = state
        .data
        .iter()
        .filter_map(|item| {
            // Match on a tuple of Options - only proceed if both are Some
            match (item.ix_count, item.fac_count) {
                (Some(ix), Some(fac)) => Some(serde_json::json!({
                    "x": ix,
                    "y": fac,
                    "label": &item.name
                })),
                _ => None, // Skip if either is None
            }
        })
        .collect();

    Json(data)
}
