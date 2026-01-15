use polars::prelude::DataFrame;
use std::env;
use tera::Tera;
use tokio::sync::RwLock;

use crate::models::Network;

/// Application configuration from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    /// Address to bind the HTTP server to.
    pub bind_address: String,
    /// Cron expression for data refresh schedule.
    pub refresh_cron: String,
}

impl Config {
    /// Creates Config from environment variables with defaults.
    pub fn from_env() -> Self {
        Self {
            bind_address: env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8201".into()),
            refresh_cron: env::var("REFRESH_CRON").unwrap_or_else(|_| "0 0 0 * * *".into()),
        }
    }
}

/// Shared application state passed to all request handlers.
#[derive(Debug)]
pub struct AppState {
    /// Template engine for rendering HTML pages.
    pub tera: Tera,
    /// Network data protected by RwLock.
    /// Stores both Vec (for templates) and DataFrame (for analytics).
    pub data: RwLock<AppData>,
}

#[derive(Debug)]
pub struct AppData {
    pub networks: Vec<Network>,
    pub df: DataFrame,
}

impl AppState {
    pub fn new(tera: Tera, networks: Vec<Network>, df: DataFrame) -> Self {
        Self {
            tera,
            data: RwLock::new(AppData { networks, df }),
        }
    }
}
