use thiserror::Error;

#[derive(Debug, Error)]
pub enum HealthPortCheckError {
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

#[derive(Debug, Error)]
pub enum HealthCheckError {
    #[error(transparent)]
    Port(#[from] HealthPortCheckError),
}
