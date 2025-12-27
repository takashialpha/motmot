use std::path::Path;

use crate::config::AppConfig;
use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tracing::{info, warn};

use crate::health::error::HealthTlsCheckError;

pub fn check_tls(config: &AppConfig) -> Result<(), HealthTlsCheckError> {
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
            .map_err(|e| HealthTlsCheckError::TlsConfigInvalid {
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

fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>, HealthTlsCheckError> {
    let file = std::fs::File::open(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            HealthTlsCheckError::CertificateNotFound {
                path: path.display().to_string(),
            }
        } else {
            HealthTlsCheckError::CertificateInvalid {
                path: path.display().to_string(),
                reason: e.to_string(),
            }
        }
    })?;

    let mut reader = std::io::BufReader::new(file);
    let mut certs = Vec::new();

    for item in rustls_pemfile::certs(&mut reader) {
        let cert = item.map_err(|e| HealthTlsCheckError::CertificateInvalid {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;
        certs.push(cert);
    }

    if certs.is_empty() {
        return Err(HealthTlsCheckError::CertificateInvalid {
            path: path.display().to_string(),
            reason: "no certificates found".into(),
        });
    }

    Ok(certs)
}

fn load_key(path: &Path) -> Result<PrivatePkcs8KeyDer<'static>, HealthTlsCheckError> {
    let file = std::fs::File::open(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            HealthTlsCheckError::PrivateKeyNotFound {
                path: path.display().to_string(),
            }
        } else {
            HealthTlsCheckError::PrivateKeyInvalid {
                path: path.display().to_string(),
                reason: e.to_string(),
            }
        }
    })?;

    let mut reader = std::io::BufReader::new(file);

    if let Some(item) = rustls_pemfile::pkcs8_private_keys(&mut reader).next() {
        let key = item.map_err(|e| HealthTlsCheckError::PrivateKeyInvalid {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;
        return Ok(key);
    }

    Err(HealthTlsCheckError::PrivateKeyInvalid {
        path: path.display().to_string(),
        reason: "no PKCS8 private key found".into(),
    })
}
