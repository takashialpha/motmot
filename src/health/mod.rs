pub mod ports;
pub mod tls;

use crate::config::AppConfig;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HealthError {
    #[error(transparent)]
    TlsError(#[from] tls::TlsError),

    #[error(transparent)]
    PortError(#[from] ports::PortError),
}

/// Entry point for health checks
pub async fn run_checks(config: &AppConfig) -> Result<(), HealthError> {
    if config.health.enabled {
        tls::check_tls(config)?;
        ports::check_port_conflicts(config)?;
        ports::check_ports_available(config).await?;
    }
    Ok(())
}
