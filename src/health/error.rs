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
pub enum HealthTlsCheckError {
    #[error("certificate file not found: {path}")]
    CertificateNotFound { path: String },

    #[error("certificate invalid: {path}, reason: {reason}")]
    CertificateInvalid { path: String, reason: String },

    #[error("private key file not found: {path}")]
    PrivateKeyNotFound { path: String },

    #[error("private key invalid: {path}, reason: {reason}")]
    PrivateKeyInvalid { path: String, reason: String },

    #[error("TLS configuration invalid for server '{server}': {reason}")]
    TlsConfigInvalid { server: String, reason: String },
}

#[derive(Debug, Error)]
pub enum HealthCheckError {
    #[error(transparent)]
    Tls(#[from] HealthTlsCheckError),

    #[error(transparent)]
    Port(#[from] HealthPortCheckError),
}
