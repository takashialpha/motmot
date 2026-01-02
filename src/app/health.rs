use super::error::AppRunError;
use crate::config::AppConfig;
use tracing::{error, info};

#[cfg(feature = "health")]
pub async fn run(config: &AppConfig) -> Result<(), AppRunError> {
    if config.health.enabled {
        info!("health_check_starting");
        crate::features::health::run_checks(config)
            .await
            .map_err(|e| {
                error!("health_check_failed: {e}");
                AppRunError::HealthCheck(format!("health_check_failed: {e}"))
            })?;
        info!("health_check_passed");
    } else {
        info!("health_check_disabled: config");
    }
    Ok(())
}

#[cfg(not(feature = "health"))]
pub async fn run(_: &AppConfig) -> Result<(), AppRunError> {
    tracing::info!("health_check_disabled: not built");
    Ok(())
}
