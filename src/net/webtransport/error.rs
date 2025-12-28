use h3::error::{ConnectionError, StreamError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WebTransportError {
    #[error("H3 connection error: {0}")]
    H3Connection(#[from] ConnectionError),

    #[error("H3 stream error: {0}")]
    H3Stream(#[from] StreamError),

    #[error("QUIC connection error: {0}")]
    QuicConnection(#[from] h3_quinn::quinn::ConnectionError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
