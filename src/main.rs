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

use crate::data::load_network_data;
use crate::fetcher::fetch_and_save_peeringdb_data;
use crate::models::{Network, Stats};

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

#[tokio::main]
async fn main() {
    // Check if we need to fetch data first
    if !std::path::Path::new("data/peeringdb/net.json").exists() {
        println!("Fetching initial data...");
        if let Err(e) = fetch_and_save_peeringdb_data().await {
            eprintln!("Error fetching initial data: {}", e);
        }
    }

    let data = load_network_data();
    let tera = match Tera::new("templates/**/*.html") {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
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

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8201").await.unwrap();
    println!("Listening on http://{}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn index(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut network_types = HashMap::new();
    let mut policy_types = HashMap::new();
    let mut scopes = HashMap::new();

    for item in &state.data {
        if let Some(t) = &item.info_type {
            *network_types.entry(t.clone()).or_insert(0) += 1;
        }
        if let Some(p) = &item.policy_general {
            *policy_types.entry(p.clone()).or_insert(0) += 1;
        }
        if let Some(s) = &item.info_scope {
            *scopes.entry(s.clone()).or_insert(0) += 1;
        }
    }

    let stats = Stats {
        total_networks: state.data.len(),
        network_types,
        policy_types,
        scopes,
    };

    let mut context = Context::new();
    context.insert("stats", &stats);
    context.insert("networks", &state.data.iter().take(10).collect::<Vec<_>>());

    match state.tera.render("dashboard.html", &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            eprintln!("Render error: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Render error",
            )
                .into_response()
        }
    }
}

async fn networks_list(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<Pagination>,
) -> impl IntoResponse {
    let page = pagination.page.unwrap_or(1);
    let per_page = pagination.per_page.unwrap_or(25);

    let total_networks = state.data.len();
    let total_pages = (total_networks + per_page - 1) / per_page;

    let start_index = (page - 1) * per_page;
    let end_index = (start_index + per_page).min(total_networks);

    let paginated_networks: Vec<_> = state.data[start_index..end_index].iter().collect();

    let mut context = Context::new();
    context.insert("networks", &paginated_networks);
    context.insert("page", &page);
    context.insert("per_page", &per_page);
    context.insert("total_pages", &total_pages);
    context.insert("total_networks", &total_networks);

    match state.tera.render("networks.html", &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            eprintln!("Render error: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Render error",
            )
                .into_response()
        }
    }
}

async fn analytics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let context = Context::new();
    match state.tera.render("analytics.html", &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            eprintln!("Render error: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Render error",
            )
                .into_response()
        }
    }
}

async fn search_networks(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let mut results = Vec::new();

    if query.asn.is_some() || query.name.is_some() {
        for network in &state.data {
            let match_asn = query.asn.is_some() && Some(network.asn) == query.asn;
            let match_name = if let Some(q_name) = &query.name {
                network.name.to_lowercase().contains(&q_name.to_lowercase())
            } else {
                false
            };

            if (query.asn.is_some() && match_asn) || (query.name.is_some() && match_name) {
                results.push(network);
            }
        }
    }

    let mut context = Context::new();
    context.insert("results", &results);
    context.insert("query_asn", &query.asn);
    context.insert("query_name", &query.name);

    match state.tera.render("search.html", &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            eprintln!("Render error: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Render error",
            )
                .into_response()
        }
    }
}

async fn api_network_types(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut network_types = HashMap::new();
    for item in &state.data {
        if let Some(t) = &item.info_type {
            *network_types.entry(t.clone()).or_insert(0) += 1;
        }
    }

    let labels: Vec<_> = network_types.keys().cloned().collect();
    let data: Vec<_> = network_types.values().cloned().collect();

    Json(serde_json::json!({
        "labels": labels,
        "data": data
    }))
}

async fn api_prefixes_distribution(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut networks = Vec::new();
    let mut ipv4 = Vec::new();
    let mut ipv6 = Vec::new();

    for item in state.data.iter().take(15) {
        if item.info_prefixes4.is_some() && item.info_prefixes6.is_some() {
            let name = if item.name.len() > 30 {
                format!("{}...", &item.name[..30])
            } else {
                item.name.clone()
            };
            networks.push(name);
            ipv4.push(item.info_prefixes4.unwrap());
            ipv6.push(item.info_prefixes6.unwrap());
        }
    }

    Json(serde_json::json!({
        "networks": networks,
        "ipv4": ipv4,
        "ipv6": ipv6
    }))
}

async fn api_ix_facility_correlation(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut data = Vec::new();
    for item in &state.data {
        if item.ix_count.is_some() && item.fac_count.is_some() {
            data.push(serde_json::json!({
                "x": item.ix_count.unwrap(),
                "y": item.fac_count.unwrap(),
                "label": item.name
            }));
        }
    }
    Json(data)
}
