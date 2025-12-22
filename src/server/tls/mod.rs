use h3_quinn::quinn::crypto::rustls::QuicServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::path::PathBuf;
use tracing::{error, info};

pub mod error;

use crate::server::tls::error::TlsConfigError;

static ALPN: &[u8] = b"h3";

pub fn load_tls_config(
    app_name: &str,
    cert_path: &PathBuf,
    key_path: &PathBuf,
) -> Result<QuicServerConfig, TlsConfigError> {
    info!(server = %app_name, "tls_config_load_started");

    let cert_bytes = std::fs::read(cert_path).map_err(|e| {
        error!(
            server = %app_name,
            path = %cert_path.display(),
            error = %e,
            "tls_cert_read_failed"
        );
        TlsConfigError::CertRead {
            path: cert_path.display().to_string(),
            source: e,
        }
    })?;

    let key_bytes = std::fs::read(key_path).map_err(|e| {
        error!(
            server = %app_name,
            path = %key_path.display(),
            error = %e,
            "tls_key_read_failed"
        );
        TlsConfigError::KeyRead {
            path: key_path.display().to_string(),
            source: e,
        }
    })?;

    let cert = CertificateDer::from(cert_bytes);

    let key = PrivateKeyDer::try_from(key_bytes).map_err(|e| {
        error!(
            server = %app_name,
            error = %e,
            "tls_key_invalid"
        );
        TlsConfigError::InvalidKey {
            message: e.to_string(),
        }
    })?;

    let mut tls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)
        .map_err(|e| {
            error!(
                server = %app_name,
                error = %e,
                "tls_cert_chain_invalid"
            );
            TlsConfigError::InvalidCertChain(e)
        })?;

    tls_config.max_early_data_size = u32::MAX;
    tls_config.alpn_protocols = vec![ALPN.into()];

    QuicServerConfig::try_from(tls_config).map_err(|e| {
        error!(
            server = %app_name,
            error = %e,
            "tls_quinn_config_failed"
        );
        TlsConfigError::QuinnConfig(e)
    })
}
