use h3::error::{ConnectionError, StreamError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("H3 connection error: {0}")]
    H3Connection(#[from] ConnectionError),

    #[error("H3 stream error: {0}")]
    H3Stream(#[from] StreamError),

    #[error("QUIC connection error: {0}")]
    Connection(#[from] h3_quinn::quinn::ConnectionError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid HTTP request: {0}")]
    InvalidRequest(String),

    #[error("route not found: {0}")]
    RouteNotFound(String),

    #[error("method not allowed: {0}")]
    MethodNotAllowed(String),

    #[error("action execution error: {0}")]
    ActionExecution(String),
}
