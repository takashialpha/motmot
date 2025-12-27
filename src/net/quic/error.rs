use crate::net::tls::error::TlsError;
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
