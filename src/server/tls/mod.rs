use h3_quinn::quinn::crypto::rustls::QuicServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::path::PathBuf;

static ALPN: &[u8] = b"h3";

pub fn load_tls_config(
    cert_path: &PathBuf,
    key_path: &PathBuf,
) -> Result<QuicServerConfig, Box<dyn std::error::Error>> {
    let cert_bytes = std::fs::read(cert_path)?;
    let key_bytes = std::fs::read(key_path)?;

    let cert = CertificateDer::from(cert_bytes);
    let key = PrivateKeyDer::try_from(key_bytes)?;

    let mut tls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)?;
    tls_config.max_early_data_size = u32::MAX;
    tls_config.alpn_protocols = vec![ALPN.into()];

    Ok(QuicServerConfig::try_from(tls_config)?)
}
