use std::error::Error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("upstream connection failed: {upstream}")]
    UpstreamConnectionFailed {
        upstream: String,
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },

    #[error("upstream request timeout after {timeout_secs}s")]
    UpstreamTimeout { timeout_secs: u64 },

    #[error("upstream returned error {status}: {message}")]
    UpstreamError { status: u16, message: String },

    #[error("invalid upstream URL: {url}")]
    InvalidUpstreamUrl { url: String },

    #[error("failed to build response: {0}")]
    ResponseBuildError(String),
}
