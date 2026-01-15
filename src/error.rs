//! Error types for NetViz application.
//!
//! This module defines custom error types using `thiserror` for clear,
//! descriptive error messages. Using custom errors instead of `Box<dyn Error>`
//! provides better type safety and more informative error handling.

use thiserror::Error;

/// Custom error type for NetViz operations.
///
/// # Rust Concepts
/// - `#[derive(Error)]` from `thiserror` auto-implements `std::error::Error`
/// - `#[error("...")]` defines the Display message for each variant
/// - `#[from]` automatically implements `From<T>` for error conversion
/// - This allows using `?` operator to convert errors automatically
#[derive(Debug, Error)]
pub enum NetVizError {
    /// Error reading or writing files.
    /// The `#[from]` attribute allows automatic conversion from `std::io::Error`.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Error parsing JSON data.
    /// Automatically converts from `serde_json::Error`.
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// Error making HTTP requests.
    /// Automatically converts from `reqwest::Error`.
    #[error("HTTP request error: {0}")]
    HttpRequest(#[from] reqwest::Error),

    /// Error with HTTP headers (e.g., invalid API key format).
    #[error("Invalid header value: {0}")]
    InvalidHeader(#[from] reqwest::header::InvalidHeaderValue),

    /// Error when API returns unexpected data format.
    #[error("Invalid API response: {0}")]
    InvalidApiResponse(String),
}
