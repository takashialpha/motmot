use std::io;

use thiserror::Error;

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
