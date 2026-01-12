use crate::config::Logging;
use crate::logging::error::LoggingError;

use tracing_subscriber::{EnvFilter, Registry, prelude::*};

pub async fn init_logging_async_systemd(cfg: &Logging) -> Result<(), LoggingError> {
    // Create the filtering layer from your config
    let filter = EnvFilter::try_new(&cfg.filter)
        .map_err(|e| LoggingError::InvalidFilter(format!("{}: {}", cfg.filter, e)))?;

    // Create the journald layer
    let journald_layer = tracing_journald::layer()
        .map_err(|e| LoggingError::InvalidFilter(format!("journald init failed: {e}")))?;

    // Compose the subscriber
    let subscriber = Registry::default().with(filter).with(journald_layer);

    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}
