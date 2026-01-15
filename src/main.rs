mod data;
mod error;
mod fetcher;
mod handlers;
mod models;
mod state;

use axum::{routing::get, Router};
use polars::prelude::*;
use std::sync::Arc;
use tera::Tera;
use tokio_cron_scheduler::{Job, JobScheduler};
use tower_http::services::ServeDir;
use tracing::{error, info, warn};

use crate::data::load_network_data;
use crate::fetcher::fetch_and_save_peeringdb_data;
use crate::models::Network;
use crate::state::{AppState, Config};

use handlers::{
    analytics, api_ix_facility_correlation, api_network_types, api_prefixes_distribution, index,
    networks_list, search_networks,
};

/// Path to the network data file from PeeringDB.
const NETWORK_DATA_PATH: &str = "data/peeringdb/net.json";

/// Converts a slice of Networks to a Polars DataFrame.
///
/// Uses serde serialization to intermediate JSON for simplicity,
/// as direct column construction is verbose.
fn networks_to_df(networks: &[Network]) -> Result<DataFrame, error::NetVizError> {
    let json = serde_json::to_string(networks)?;
    let cursor = std::io::Cursor::new(json);
    let df = JsonReader::new(cursor).finish()?;
    Ok(df)
}

async fn refresh_data(state: Arc<AppState>) {
    info!("Starting scheduled data refresh from PeeringDB...");

    if let Err(e) = fetch_and_save_peeringdb_data().await {
        error!("Failed to fetch data from PeeringDB: {}", e);
        return;
    }

    match tokio::task::spawn_blocking(load_network_data).await {
        Ok(result) => match result {
            Ok(new_data) => {
                let count = new_data.len();
                // Create DataFrame from new data
                match networks_to_df(&new_data) {
                    Ok(new_df) => {
                        let mut data_guard = state.data.write().await;
                        data_guard.networks = new_data;
                        data_guard.df = new_df;
                        info!("Data refresh complete: loaded {} networks", count);
                    }
                    Err(e) => {
                        error!("Failed to create DataFrame from refreshed data: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to load refreshed data: {}", e);
            }
        },
        Err(e) => {
            error!("Failed to execute background load task: {}", e);
        }
    }
}

async fn start_scheduler(
    cron_expr: &str,
    state: Arc<AppState>,
) -> Result<JobScheduler, Box<dyn std::error::Error + Send + Sync>> {
    let scheduler = JobScheduler::new().await?;
    let state_clone = state.clone();

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

    // Fetch data if needed
    if !std::path::Path::new(NETWORK_DATA_PATH).exists() {
        info!("Fetching initial data from PeeringDB...");
        if let Err(e) = fetch_and_save_peeringdb_data().await {
            error!("Failed to fetch initial data: {}", e);
        }
    }

    // Load initial data
    let (networks, df) = match load_network_data() {
        Ok(d) => {
            info!("Loaded {} networks from data file", d.len());
            match networks_to_df(&d) {
                Ok(df) => (d, df),
                Err(e) => {
                    error!("Failed to create initial DataFrame: {}", e);
                    std::process::exit(1);
                }
            }
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

    let state = Arc::new(AppState::new(tera, networks, df));

    // Start scheduler
    let _scheduler = match start_scheduler(&config.refresh_cron, state.clone()).await {
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
        .nest_service("/assets", ServeDir::new("assets"))
        .with_state(state)
        .layer(
            tower_http::trace::TraceLayer::new_for_http()
                .on_request(tower_http::trace::DefaultOnRequest::new().level(tracing::Level::INFO))
                .on_response(
                    tower_http::trace::DefaultOnResponse::new().level(tracing::Level::INFO),
                )
                .on_failure(
                    tower_http::trace::DefaultOnFailure::new().level(tracing::Level::ERROR),
                ),
        );

    let listener = tokio::net::TcpListener::bind(&config.bind_address)
        .await
        .expect("TCP listener must bind to configured address");

    info!("Server listening on http://{}", config.bind_address);
    info!("Data refresh scheduled: {}", config.refresh_cron);

    if let Err(e) = axum::serve(listener, app).await {
        error!("Server error: {}", e);
        std::process::exit(1);
    }
}
