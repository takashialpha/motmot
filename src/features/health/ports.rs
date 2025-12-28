use std::collections::HashMap;
use tokio::net::TcpListener;
use tracing::info;

use crate::config::AppConfig;
use crate::features::health::error::HealthPortCheckError;

pub fn check_port_conflicts(config: &AppConfig) -> Result<(), HealthPortCheckError> {
    let mut map: HashMap<(String, u16), Vec<String>> = HashMap::new();

    for (name, server) in &config.servers {
        map.entry((server.host.clone(), server.port))
            .or_default()
            .push(name.clone());
    }

    for ((host, port), servers) in map {
        if servers.len() > 1 {
            return Err(HealthPortCheckError::PortConflict {
                host,
                port,
                servers,
            });
        }
    }

    info!("health_check_no_port_conflicts");
    Ok(())
}

pub async fn check_ports_available(config: &AppConfig) -> Result<(), HealthPortCheckError> {
    for (name, server) in &config.servers {
        let addr = format!("{}:{}", server.host, server.port);

        let listener =
            TcpListener::bind(&addr)
                .await
                .map_err(|e| HealthPortCheckError::PortNotAvailable {
                    host: server.host.clone(),
                    port: server.port,
                    source: e,
                })?;

        drop(listener);

        info!(
            server = %name,
            host = %server.host,
            port = server.port,
            "health_check_port_available"
        );
    }

    Ok(())
}
