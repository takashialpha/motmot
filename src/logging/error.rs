use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LoggingError {
    #[error("invalid log filter: {0}")]
    InvalidFilter(String),

    #[error("failed to open log file: {0}")]
    Io(#[from] io::Error),

    #[error("failed to set global subscriber: {0}")]
    SetSubscriber(#[from] tracing::subscriber::SetGlobalDefaultError),
}
