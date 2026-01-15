//! Custom error types for NetViz application.

use thiserror::Error;

/// Error types for NetViz operations.
#[derive(Debug, Error)]
pub enum NetVizError {
    /// File I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON parsing error.
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// HTTP request error.
    #[error("HTTP request error: {0}")]
    HttpRequest(#[from] reqwest::Error),

    /// Invalid HTTP header value.
    #[error("Invalid header value: {0}")]
    InvalidHeader(#[from] reqwest::header::InvalidHeaderValue),

    /// Unexpected API response format.
    #[error("Invalid API response: {0}")]
    InvalidApiResponse(String),
}
