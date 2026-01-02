use thiserror::Error;

use app_base::{AppError, config::ConfigError};

#[derive(Debug, Error)]
pub enum AppRunError {
    #[error("failed to initialize tokio runtime")]
    RuntimeInit(#[source] std::io::Error),

    #[error("logging initialization failed")]
    LoggingInit(String),

    #[error("health check failed")]
    HealthCheck(String),
}

impl From<AppRunError> for AppError {
    fn from(err: AppRunError) -> Self {
        match err {
            AppRunError::RuntimeInit(e) => AppError::from(ConfigError::Io(e)),
            AppRunError::LoggingInit(msg) | AppRunError::HealthCheck(msg) => {
                AppError::from(ConfigError::Io(std::io::Error::other(msg)))
            }
        }
    }
}
