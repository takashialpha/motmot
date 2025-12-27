pub mod error;
pub mod ports;
pub mod tls;

use crate::config::AppConfig;
use crate::health::error::HealthCheckError;

pub async fn run_checks(config: &AppConfig) -> Result<(), HealthCheckError> {
    if config.health.enabled {
        tls::check_tls(config)?;
        ports::check_port_conflicts(config)?;
        ports::check_ports_available(config).await?;
    }
    Ok(())
}
