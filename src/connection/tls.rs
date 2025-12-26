use std::path::{Path, PathBuf};

use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tracing::{info, warn};

use super::error::TlsError;

/// Load TLS configuration from files or generate self-signed certificate
///
/// Behavior:
/// - Both cert_path and key_path provided: Load from files
/// - One or both missing: Generate self-signed certificate pair
/// - Generated certificates are saved to /etc/motmot/ssl/generated/
pub async fn load_or_generate(
    server_name: &str,
    cert_path: Option<&PathBuf>,
    key_path: Option<&PathBuf>,
) -> Result<rustls::ServerConfig, TlsError> {
    match (cert_path, key_path) {
        // Both paths provided - load from disk
        (Some(cert), Some(key)) => {
            info!(
                server = server_name,
                cert = %cert.display(),
                key = %key.display(),
                "tls_load_from_file"
            );
            load_from_files(server_name, cert, key).await
        }

        // One or both missing - generate self-signed certificate
        _ => {
            if cert_path.is_some() || key_path.is_some() {
                warn!(
                    server = server_name,
                    cert_provided = cert_path.is_some(),
                    key_provided = key_path.is_some(),
                    "tls_incomplete_config_generating"
                );
            }

            let gen_dir = PathBuf::from("/etc/motmot/ssl/generated");
            let cert_path = gen_dir.join(format!("{}.cert", server_name));
            let key_path = gen_dir.join(format!("{}.key", server_name));

            // Check if already generated and valid
            if cert_path.exists() && key_path.exists() {
                info!(
                    server = server_name,
                    cert = %cert_path.display(),
                    "tls_checking_existing_generated"
                );

                if let Ok(config) = load_from_files(server_name, &cert_path, &key_path).await {
                    info!(server = server_name, "tls_using_existing_generated");
                    return Ok(config);
                }

                warn!(server = server_name, "tls_existing_invalid_regenerating");
            }

            // Generate new self-signed certificate
            warn!(
                server = server_name,
                cert = %cert_path.display(),
                key = %key_path.display(),
                "tls_generating_self_signed"
            );

            generate_and_save(server_name, &cert_path, &key_path).await
        }
    }
}

/// Load TLS configuration from certificate and key files
async fn load_from_files(
    server_name: &str,
    cert_path: &Path,
    key_path: &Path,
) -> Result<rustls::ServerConfig, TlsError> {
    // Read certificate file
    let cert_pem = tokio::fs::read(cert_path)
        .await
        .map_err(|e| TlsError::CertificateRead {
            path: cert_path.display().to_string(),
            source: e,
        })?;

    // Read private key file
    let key_pem = tokio::fs::read(key_path)
        .await
        .map_err(|e| TlsError::PrivateKeyRead {
            path: key_path.display().to_string(),
            source: e,
        })?;

    // Parse certificates
    let certs = rustls_pemfile::certs(&mut cert_pem.as_slice())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| TlsError::InvalidCertificate {
            path: cert_path.display().to_string(),
        })?;

    if certs.is_empty() {
        return Err(TlsError::InvalidCertificate {
            path: cert_path.display().to_string(),
        });
    }

    // Parse private key
    let key = rustls_pemfile::private_key(&mut key_pem.as_slice())
        .map_err(|_| TlsError::InvalidPrivateKey {
            path: key_path.display().to_string(),
        })?
        .ok_or_else(|| TlsError::InvalidPrivateKey {
            path: key_path.display().to_string(),
        })?;

    // Build rustls ServerConfig
    build_server_config(certs, key)
}

/// Generate self-signed certificate and save to disk
async fn generate_and_save(
    server_name: &str,
    cert_path: &Path,
    key_path: &Path,
) -> Result<rustls::ServerConfig, TlsError> {
    // Generate self-signed certificate using rcgen
    let cert = rcgen::generate_simple_self_signed(vec![server_name.to_string()])
        .map_err(|e| TlsError::Generation(e.to_string()))?;

    let cert_pem = cert.cert.pem();
    let key_pem = cert.signing_key.serialize_pem();

    // Ensure directory exists
    if let Some(parent) = cert_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| TlsError::CertificateWrite {
                path: parent.display().to_string(),
                source: e,
            })?;
    }

    // Write certificate to disk
    tokio::fs::write(cert_path, cert_pem.as_bytes())
        .await
        .map_err(|e| TlsError::CertificateWrite {
            path: cert_path.display().to_string(),
            source: e,
        })?;

    // Write private key to disk
    tokio::fs::write(key_path, key_pem.as_bytes())
        .await
        .map_err(|e| TlsError::PrivateKeyWrite {
            path: key_path.display().to_string(),
            source: e,
        })?;

    warn!(
        server = server_name,
        cert = %cert_path.display(),
        "tls_self_signed_saved_not_for_production"
    );

    // Load the newly generated certificate
    load_from_files(server_name, cert_path, key_path).await
}

/// Build rustls ServerConfig from certificates and private key
fn build_server_config(
    certs: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
) -> Result<rustls::ServerConfig, TlsError> {
    rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| TlsError::ConfigCreation(e.to_string()))
}
