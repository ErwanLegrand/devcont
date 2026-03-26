// Copyright (c) 2025 Erwan Patrick Legrand

use thiserror::Error;

/// Result type alias for operations that can fail
pub type Result<T> = std::result::Result<T, Error>;

/// Comprehensive error type for the rust-dev-template crate
#[derive(Debug, Error)]
pub enum Error {
    /// I/O operation failed
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization or deserialization failed
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// Parsing failed
    #[error("Parse error: {0}")]
    Parse(String),

    /// Validation failed
    #[error("Validation error: {0}")]
    Validation(String),

    /// Custom error with message
    #[error("Custom error: {message}")]
    Custom { message: String },

    /// Operation timed out
    #[error("Timeout error: {0}")]
    Timeout(String),

    /// Network operation failed
    #[error("Network error: {0}")]
    Network(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
}

/// Convert any error to our Error type
impl From<Box<dyn std::error::Error + Send + Sync>> for Error {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::Custom {
            message: err.to_string(),
        }
    }
}
