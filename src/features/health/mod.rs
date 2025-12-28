pub mod error;
pub mod ports;

use crate::config::AppConfig;
use crate::features::health::error::HealthCheckError;

pub async fn run_checks(config: &AppConfig) -> Result<(), HealthCheckError> {
    if config.health.enabled {
        ports::check_port_conflicts(config)?;
        ports::check_ports_available(config).await?;
    }
    Ok(())
}
