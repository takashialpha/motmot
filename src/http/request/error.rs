use thiserror::Error;

#[derive(Debug, Error)]
pub enum RequestError {
    #[error(transparent)]
    Connection(#[from] h3::error::ConnectionError),

    #[error(transparent)]
    Stream(#[from] h3::error::StreamError),

    #[error(transparent)]
    Response(#[from] crate::http::response::error::ResponseError),

    #[error("invalid server configuration: {0}")]
    Config(String),
}
