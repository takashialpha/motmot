use h3::error::{ConnectionError, StreamError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("H3 connection error: {0}")]
    H3Connection(#[from] ConnectionError),

    #[error("H3 stream error: {0}")]
    H3Stream(#[from] StreamError),

    #[error("QUIC connection error: {0}")]
    Quic(#[from] quinn::ConnectionError),

    #[error("webtransport error: {0}")]
    WebTransport(String),

    #[error("server configuration missing for {0}")]
    MissingServerConfig(String),
}
