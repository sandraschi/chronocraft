//! Error types for ChronoCraft L1 core.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChronoError {
    #[error("Stock, flow, link, parameter, or scenario not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("SQLite storage error: {0}")]
    Storage(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}
