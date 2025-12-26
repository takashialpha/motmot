use std::{collections::HashMap, fs::File, io::BufReader, path::Path};

use thiserror::Error;
use tokio::net::TcpListener;
use tracing::{info, warn};

use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

use crate::config::{Action, AppConfig};

#[derive(Debug, Error)]
pub enum HealthError {
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

    #[error("directory not found: {path}")]
    DirectoryNotFound { path: String },

    #[error("directory not accessible: {path}, reason: {source}")]
    DirectoryNotAccessible {
        path: String,
        source: std::io::Error,
    },

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

    #[error("insufficient permissions to write to: {path}")]
    InsufficientPermissions { path: String },
}

/// Entry point
pub async fn run_checks(config: &AppConfig) -> Result<(), HealthError> {
    let health = &config.health;

    if health.check_certs {
        check_certificates_quic(config)?;
    }

    if health.check_directories {
        check_directories(config).await?;
    }

    if health.check_ports {
        check_port_conflicts(config)?;
        check_ports_available(config).await?;
    }

    Ok(())
}

//
// ─── TLS / CERT VALIDATION ─────────────────────────────────────────────────────
//

fn check_certificates_quic(config: &AppConfig) -> Result<(), HealthError> {
    for (name, server) in &config.servers {
        let (cert_path, key_path) = match (&server.cert_path, &server.key_path) {
            (Some(c), Some(k)) => (c, k),
            _ => {
                warn!(
                    server = %name,
                    "cert/key missing; will rely on generation"
                );
                continue;
            }
        };

        let certs = load_certs(cert_path)?;
        let key = load_key(key_path)?;

        let mut tls = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, PrivateKeyDer::Pkcs8(key))
            .map_err(|e| HealthError::TlsConfigInvalid {
                server: name.clone(),
                reason: e.to_string(),
            })?;

        // REQUIRED for QUIC / HTTP3
        tls.alpn_protocols.push(b"h3".to_vec());

        info!(
            server = %name,
            cert = %cert_path.display(),
            key = %key_path.display(),
            "health_check_tls_quic_valid"
        );
    }

    Ok(())
}

fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>, HealthError> {
    let file = File::open(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            HealthError::CertificateNotFound {
                path: path.display().to_string(),
            }
        } else {
            HealthError::CertificateInvalid {
                path: path.display().to_string(),
                reason: e.to_string(),
            }
        }
    })?;

    let mut reader = BufReader::new(file);
    let mut certs = Vec::new();

    for item in rustls_pemfile::certs(&mut reader) {
        let cert = item.map_err(|e| HealthError::CertificateInvalid {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;
        certs.push(cert);
    }

    if certs.is_empty() {
        return Err(HealthError::CertificateInvalid {
            path: path.display().to_string(),
            reason: "no certificates found".into(),
        });
    }

    Ok(certs)
}

fn load_key(path: &Path) -> Result<PrivatePkcs8KeyDer<'static>, HealthError> {
    let file = File::open(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            HealthError::PrivateKeyNotFound {
                path: path.display().to_string(),
            }
        } else {
            HealthError::PrivateKeyInvalid {
                path: path.display().to_string(),
                reason: e.to_string(),
            }
        }
    })?;

    let mut reader = BufReader::new(file);

    for item in rustls_pemfile::pkcs8_private_keys(&mut reader) {
        let key = item.map_err(|e| HealthError::PrivateKeyInvalid {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;
        return Ok(key);
    }

    Err(HealthError::PrivateKeyInvalid {
        path: path.display().to_string(),
        reason: "no PKCS8 private key found".into(),
    })
}

//
// ─── FILESYSTEM CHECKS ──────────────────────────────────────────────────────────
//

async fn check_directories(config: &AppConfig) -> Result<(), HealthError> {
    for (server_name, server) in &config.servers {
        for (route_path, route_config) in &server.routes {
            for action in [
                &route_config.get,
                &route_config.post,
                &route_config.put,
                &route_config.delete,
                &route_config.patch,
                &route_config.head,
                &route_config.options,
                &route_config.fallback,
            ]
            .iter()
            .filter_map(|a| a.as_ref())
            {
                if let Action::Static { directory, .. } = action {
                    let meta = tokio::fs::metadata(directory).await.map_err(|e| {
                        if e.kind() == std::io::ErrorKind::NotFound {
                            HealthError::DirectoryNotFound {
                                path: directory.display().to_string(),
                            }
                        } else {
                            HealthError::DirectoryNotAccessible {
                                path: directory.display().to_string(),
                                source: e,
                            }
                        }
                    })?;

                    if !meta.is_dir() {
                        return Err(HealthError::DirectoryNotAccessible {
                            path: directory.display().to_string(),
                            source: std::io::Error::new(
                                std::io::ErrorKind::InvalidInput,
                                "not a directory",
                            ),
                        });
                    }

                    info!(
                        server = %server_name,
                        route = %route_path,
                        directory = %directory.display(),
                        "health_check_directory_accessible"
                    );
                }
            }
        }
    }

    Ok(())
}

//
// ─── PORT CHECKS ────────────────────────────────────────────────────────────────
//

fn check_port_conflicts(config: &AppConfig) -> Result<(), HealthError> {
    let mut map: HashMap<(String, u16), Vec<String>> = HashMap::new();

    for (name, server) in &config.servers {
        map.entry((server.host.clone(), server.port))
            .or_default()
            .push(name.clone());
    }

    for ((host, port), servers) in map {
        if servers.len() > 1 {
            return Err(HealthError::PortConflict {
                host,
                port,
                servers,
            });
        }
    }

    info!("health_check_no_port_conflicts");
    Ok(())
}

async fn check_ports_available(config: &AppConfig) -> Result<(), HealthError> {
    for (name, server) in &config.servers {
        let addr = format!("{}:{}", server.host, server.port);

        let listener =
            TcpListener::bind(&addr)
                .await
                .map_err(|e| HealthError::PortNotAvailable {
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
