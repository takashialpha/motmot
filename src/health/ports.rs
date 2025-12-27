use std::collections::HashMap;
use thiserror::Error;
use tokio::net::TcpListener;
use tracing::info;

use crate::config::AppConfig;

#[derive(Debug, Error)]
pub enum PortError {
    #[error("port conflict: {host}:{port} used by servers: {}", servers.join(", "))]
    PortConflict {
        host: String,
        port: u16,
        servers: Vec<String>,
    },

    #[error("port not available: {host}:{port}, reason: {source}")]
    PortNotAvailable {
        host: String,
        port: u16,
        source: std::io::Error,
    },
}

pub fn check_port_conflicts(config: &AppConfig) -> Result<(), PortError> {
    let mut map: HashMap<(String, u16), Vec<String>> = HashMap::new();

    for (name, server) in &config.servers {
        map.entry((server.host.clone(), server.port))
            .or_default()
            .push(name.clone());
    }

    for ((host, port), servers) in map {
        if servers.len() > 1 {
            return Err(PortError::PortConflict {
                host,
                port,
                servers,
            });
        }
    }

    info!("health_check_no_port_conflicts");
    Ok(())
}

pub async fn check_ports_available(config: &AppConfig) -> Result<(), PortError> {
    for (name, server) in &config.servers {
        let addr = format!("{}:{}", server.host, server.port);

        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| PortError::PortNotAvailable {
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
