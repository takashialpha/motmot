use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error("server '{0}' not found in configuration")]
    ServerNotFound(String),

    #[error("failed to resolve address for host '{host}': {source}")]
    AddressResolution { host: String, source: io::Error },

    #[error("no IPv6 address found for host '{host}'")]
    NoIpv6Address { host: String },

    #[error("failed to create socket: {0}")]
    SocketCreation(io::Error),

    #[error("failed to configure socket: {0}")]
    SocketConfiguration(io::Error),

    #[error("failed to bind socket: {0}")]
    SocketBind(io::Error),

    #[error("failed to create QUIC endpoint: {0}")]
    EndpointCreation(io::Error),

    #[error("TLS configuration error: {0}")]
    Tls(#[from] TlsError),
}

#[derive(Debug, Error)]
pub enum TlsError {
    #[error("failed to read certificate from '{path}': {source}")]
    CertificateRead { path: String, source: io::Error },

    #[error("failed to read private key from '{path}': {source}")]
    PrivateKeyRead { path: String, source: io::Error },

    #[error("invalid certificate format in '{path}'")]
    InvalidCertificate { path: String },

    #[error("invalid private key format in '{path}'")]
    InvalidPrivateKey { path: String },

    #[error("failed to generate self-signed certificate: {0}")]
    Generation(String),

    #[error("failed to write generated certificate to '{path}': {source}")]
    CertificateWrite { path: String, source: io::Error },

    #[error("failed to write generated private key to '{path}': {source}")]
    PrivateKeyWrite { path: String, source: io::Error },

    #[error("failed to create TLS configuration: {0}")]
    ConfigCreation(String),
}
