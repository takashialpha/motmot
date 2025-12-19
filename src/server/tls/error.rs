use quinn::crypto::rustls::QuicServerConfig;
use std::{error::Error, fmt};

#[derive(Debug)]
pub enum TlsConfigError {
    CertRead {
        path: String,
        source: std::io::Error,
    },
    KeyRead {
        path: String,
        source: std::io::Error,
    },
    /// The PrivateKeyDer::try_from(Vec<u8>) error is `&'static str` in many versions;
    /// we store it as string for user-friendly messages.
    InvalidKey {
        message: String,
    },
    InvalidCertChain(rustls::Error),
    /// Use the exact associated error type produced by `QuicServerConfig::try_from`.
    QuinnConfig(<QuicServerConfig as TryFrom<rustls::ServerConfig>>::Error),
}

impl fmt::Display for TlsConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TlsConfigError::CertRead { path, .. } => {
                write!(f, "failed to read TLS certificate at {}", path)
            }
            TlsConfigError::KeyRead { path, .. } => {
                write!(f, "failed to read TLS private key at {}", path)
            }
            TlsConfigError::InvalidKey { message } => {
                write!(f, "private key parse error: {}", message)
            }
            TlsConfigError::InvalidCertChain(e) => {
                write!(f, "invalid certificate chain / private key: {}", e)
            }
            TlsConfigError::QuinnConfig(e) => {
                write!(f, "failed to construct QUIC TLS config: {}", e)
            }
        }
    }
}

impl Error for TlsConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            TlsConfigError::CertRead { source, .. } => Some(source),
            TlsConfigError::KeyRead { source, .. } => Some(source),
            TlsConfigError::InvalidKey { .. } => None,
            TlsConfigError::InvalidCertChain(e) => Some(e),
            TlsConfigError::QuinnConfig(e) => Some(e),
        }
    }
}
