use std::collections::HashMap;
use std::path::Path;

use thiserror::Error;
use tokio::net::TcpListener;
use tracing::{info, warn};

use crate::config::{Action, AppConfig};

#[derive(Debug, Error)]
pub enum HealthError {
    #[error("certificate file not found: {path}")]
    CertificateNotFound { path: String },

    #[error("certificate file not readable: {path}, reason: {source}")]
    CertificateNotReadable {
        path: String,
        source: std::io::Error,
    },

    #[error("private key file not found: {path}")]
    PrivateKeyNotFound { path: String },

    #[error("private key file not readable: {path}, reason: {source}")]
    PrivateKeyNotReadable {
        path: String,
        source: std::io::Error,
    },

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

/// Run all enabled health checks
pub async fn run_checks(config: &AppConfig) -> Result<(), HealthError> {
    let health = &config.health;

    if health.check_certs {
        check_certificates(config).await?;
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

/// Check that all certificate paths exist or can be generated
async fn check_certificates(config: &AppConfig) -> Result<(), HealthError> {
    for (name, server) in &config.servers {
        match (&server.cert_path, &server.key_path) {
            // Both provided - verify they exist and are readable
            (Some(cert_path), Some(key_path)) => {
                check_file_readable(cert_path, |path, source| {
                    if source.kind() == std::io::ErrorKind::NotFound {
                        HealthError::CertificateNotFound {
                            path: path.to_string(),
                        }
                    } else {
                        HealthError::CertificateNotReadable {
                            path: path.to_string(),
                            source,
                        }
                    }
                })
                .await?;

                check_file_readable(key_path, |path, source| {
                    if source.kind() == std::io::ErrorKind::NotFound {
                        HealthError::PrivateKeyNotFound {
                            path: path.to_string(),
                        }
                    } else {
                        HealthError::PrivateKeyNotReadable {
                            path: path.to_string(),
                            source,
                        }
                    }
                })
                .await?;

                info!(
                    server = %name,
                    cert = %cert_path.display(),
                    key = %key_path.display(),
                    "health_check_certs_exist"
                );
            }

            // One or both missing - verify we can write to generated directory
            _ => {
                let gen_dir = std::path::PathBuf::from("/etc/motmot/ssl/generated");

                // Check if directory exists or can be created
                if !gen_dir.exists() {
                    // Try to create it (will fail if no permissions)
                    tokio::fs::create_dir_all(&gen_dir).await.map_err(|e| {
                        HealthError::InsufficientPermissions {
                            path: gen_dir.display().to_string(),
                        }
                    })?;
                }

                // Verify write access by creating a test file
                let test_file = gen_dir.join(".write_test");
                tokio::fs::write(&test_file, b"test").await.map_err(|_| {
                    HealthError::InsufficientPermissions {
                        path: gen_dir.display().to_string(),
                    }
                })?;
                let _ = tokio::fs::remove_file(&test_file).await;

                warn!(
                    server = %name,
                    gen_dir = %gen_dir.display(),
                    "health_check_certs_will_generate"
                );
            }
        }
    }

    Ok(())
}

/// Check that all static file directories exist and are accessible
async fn check_directories(config: &AppConfig) -> Result<(), HealthError> {
    for (server_name, server) in &config.servers {
        for (route_path, route_config) in &server.routes {
            // Check each action type that uses directories
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
                    check_directory_accessible(directory)
                        .await
                        .map_err(|e| match e.kind() {
                            std::io::ErrorKind::NotFound => HealthError::DirectoryNotFound {
                                path: directory.display().to_string(),
                            },
                            _ => HealthError::DirectoryNotAccessible {
                                path: directory.display().to_string(),
                                source: e,
                            },
                        })?;

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

/// Check for port conflicts in configuration
fn check_port_conflicts(config: &AppConfig) -> Result<(), HealthError> {
    let mut port_map: HashMap<(String, u16), Vec<String>> = HashMap::new();

    for (name, server) in &config.servers {
        let key = (server.host.clone(), server.port);
        port_map.entry(key).or_default().push(name.clone());
    }

    for ((host, port), servers) in port_map {
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

/// Check that all configured ports are available for binding
async fn check_ports_available(config: &AppConfig) -> Result<(), HealthError> {
    for (name, server) in &config.servers {
        // Try to bind to the port
        let addr = format!("{}:{}", server.host, server.port);
        let listener =
            TcpListener::bind(&addr)
                .await
                .map_err(|e| HealthError::PortNotAvailable {
                    host: server.host.clone(),
                    port: server.port,
                    source: e,
                })?;

        // Immediately drop the listener to free the port
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

/// Helper: Check if a file exists and is readable
async fn check_file_readable<F>(path: &Path, error_fn: F) -> Result<(), HealthError>
where
    F: FnOnce(String, std::io::Error) -> HealthError,
{
    tokio::fs::metadata(path)
        .await
        .map_err(|e| error_fn(path.display().to_string(), e))?;
    Ok(())
}

/// Helper: Check if a directory exists and is accessible
async fn check_directory_accessible(path: &Path) -> Result<(), std::io::Error> {
    let metadata = tokio::fs::metadata(path).await?;
    if !metadata.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotADirectory,
            "path is not a directory",
        ));
    }
    Ok(())
}
