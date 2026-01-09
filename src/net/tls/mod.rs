pub mod error;

use std::path::{Path, PathBuf};

use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
use tracing::{info, warn};

use error::TlsError;

// gen self signed or load if given correctly.
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

async fn load_from_files(
    server_name: &str,
    cert_path: &Path,
    key_path: &Path,
) -> Result<rustls::ServerConfig, TlsError> {
    let cert_pem = match tokio::fs::read(cert_path).await {
        Ok(bytes) => bytes,
        Err(e) => {
            warn!(
                server = server_name,
                cert = %cert_path.display(),
                "certs_not_well_configured: failed to read certificate, not going to listen"
            );
            return Err(TlsError::CertificateRead {
                path: cert_path.display().to_string(),
                source: e,
            });
        }
    };

    let key_pem = match tokio::fs::read(key_path).await {
        Ok(bytes) => bytes,
        Err(e) => {
            warn!(
                server = server_name,
                key = %key_path.display(),
                "certs_not_well_configured: failed to read key, not going to listen"
            );
            return Err(TlsError::PrivateKeyRead {
                path: key_path.display().to_string(),
                source: e,
            });
        }
    };

    let certs = match CertificateDer::pem_reader_iter(&mut cert_pem.as_slice())
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(certs) if !certs.is_empty() => certs,
        _ => {
            warn!(
                server = server_name,
                cert = %cert_path.display(),
                "certs_not_well_configured: invalid or empty certificate, not going to listen"
            );
            return Err(TlsError::InvalidCertificate {
                path: cert_path.display().to_string(),
            });
        }
    };

    let key = match PrivateKeyDer::from_pem_reader(&mut key_pem.as_slice()) {
        Ok(k) => k,
        Err(_) => {
            warn!(
                server = server_name,
                key = %key_path.display(),
                "certs_not_well_configured: invalid private key, not going to listen"
            );
            return Err(TlsError::InvalidPrivateKey {
                path: key_path.display().to_string(),
            });
        }
    };

    build_server_config(server_name, certs, key)
}

async fn generate_and_save(
    server_name: &str,
    cert_path: &Path,
    key_path: &Path,
) -> Result<rustls::ServerConfig, TlsError> {
    let cert = rcgen::generate_simple_self_signed(vec![server_name.to_string()])
        .map_err(|e| TlsError::Generation(e.to_string()))?;

    let cert_pem = cert.cert.pem();
    let key_pem = cert.signing_key.serialize_pem();

    if let Some(parent) = cert_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| TlsError::CertificateWrite {
                path: parent.display().to_string(),
                source: e,
            })?;
    }

    tokio::fs::write(cert_path, cert_pem.as_bytes())
        .await
        .map_err(|e| TlsError::CertificateWrite {
            path: cert_path.display().to_string(),
            source: e,
        })?;

    tokio::fs::write(key_path, key_pem.as_bytes())
        .await
        .map_err(|e| TlsError::PrivateKeyWrite {
            path: key_path.display().to_string(),
            source: e,
        })?;

    warn!(
        server = server_name,
        cert = %cert_path.display(),
        "tls_self_signed_saved"
    );

    load_from_files(server_name, cert_path, key_path).await
}

fn build_server_config(
    server_name: &str,
    certs: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
) -> Result<rustls::ServerConfig, TlsError> {
    match rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
    {
        Ok(mut config) => {
            config.alpn_protocols = vec![b"h3".to_vec()];
            Ok(config)
        }
        Err(e) => {
            warn!(
                server = server_name,
                "certs_not_well_configured: failed to create server config, not going to listen, error = %e"
            );
            Err(TlsError::ConfigCreation(e.to_string()))
        }
    }
}
