use thiserror::Error;

#[derive(Debug, Error)]
pub enum ActionError {
    #[error("action not yet implemented: {0}")]
    NotImplemented(String),

    #[error("file not found: {path}")]
    FileNotFound { path: String },

    #[error("path error: {0}")]
    Path(#[from] crate::tools::PathError),

    #[error("proxy error: {0}")]
    Proxy(#[from] crate::proxy::ProxyError),

    #[error("invalid JSON body: {0}")]
    InvalidJson(String),

    #[error("invalid response: {0}")]
    InvalidResponse(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
