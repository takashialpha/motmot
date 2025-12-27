use std::path::Path;

use crate::config::AppConfig;
use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use thiserror::Error;
use tracing::{info, warn};

#[derive(Debug, Error)]
pub enum TlsError {
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

pub fn check_tls(config: &AppConfig) -> Result<(), TlsError> {
    for (name, server) in &config.servers {
        let tls = match &server.tls {
            Some(t) => t,
            None => {
                warn!(server = %name, "TLS not configured; will rely on generation");
                continue;
            }
        };

        let certs = load_certs(&tls.cert)?;
        let key = load_key(&tls.key)?;

        let mut tls_cfg = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, PrivateKeyDer::Pkcs8(key))
            .map_err(|e| TlsError::TlsConfigInvalid {
                server: name.clone(),
                reason: e.to_string(),
            })?;

        tls_cfg.alpn_protocols.push(b"h3".to_vec());

        info!(
            server = %name,
            cert = %tls.cert.display(),
            key = %tls.key.display(),
            "health_check_tls_quic_valid"
        );
    }

    Ok(())
}

fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>, TlsError> {
    let file = std::fs::File::open(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            TlsError::CertificateNotFound {
                path: path.display().to_string(),
            }
        } else {
            TlsError::CertificateInvalid {
                path: path.display().to_string(),
                reason: e.to_string(),
            }
        }
    })?;

    let mut reader = std::io::BufReader::new(file);
    let mut certs = Vec::new();

    for item in rustls_pemfile::certs(&mut reader) {
        let cert = item.map_err(|e| TlsError::CertificateInvalid {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;
        certs.push(cert);
    }

    if certs.is_empty() {
        return Err(TlsError::CertificateInvalid {
            path: path.display().to_string(),
            reason: "no certificates found".into(),
        });
    }

    Ok(certs)
}

fn load_key(path: &Path) -> Result<PrivatePkcs8KeyDer<'static>, TlsError> {
    let file = std::fs::File::open(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            TlsError::PrivateKeyNotFound {
                path: path.display().to_string(),
            }
        } else {
            TlsError::PrivateKeyInvalid {
                path: path.display().to_string(),
                reason: e.to_string(),
            }
        }
    })?;

    let mut reader = std::io::BufReader::new(file);

    if let Some(item) = rustls_pemfile::pkcs8_private_keys(&mut reader).next() {
        let key = item.map_err(|e| TlsError::PrivateKeyInvalid {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;
        return Ok(key);
    }

    Err(TlsError::PrivateKeyInvalid {
        path: path.display().to_string(),
        reason: "no PKCS8 private key found".into(),
    })
}
