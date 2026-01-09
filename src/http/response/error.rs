use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResponseError {
    #[error(transparent)]
    Stream(#[from] h3::error::StreamError),
}
