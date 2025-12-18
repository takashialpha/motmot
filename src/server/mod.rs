use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use bytes::{Bytes, BytesMut};
use http::StatusCode;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio::{fs::File, io::AsyncReadExt};
use tracing::error;

use h3::server::RequestResolver;
use h3_quinn::quinn::{self, crypto::rustls::QuicServerConfig};

static ALPN: &[u8] = b"h3";

pub struct ServerConfig {
    pub root: Arc<Option<PathBuf>>,
    pub listen: SocketAddr,
    pub cert: Arc<Vec<u8>>,
    pub key: Arc<Vec<u8>>,
}

pub async fn run_server(cfg: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    let cert = CertificateDer::from((*cfg.cert).clone());
    let key = PrivateKeyDer::try_from((*cfg.key).clone())?;

    let mut tls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)?;

    tls_config.max_early_data_size = u32::MAX;
    tls_config.alpn_protocols = vec![ALPN.into()];

    let server_config =
        quinn::ServerConfig::with_crypto(Arc::new(QuicServerConfig::try_from(tls_config)?));

    let endpoint = quinn::Endpoint::server(server_config, cfg.listen)?;
    tracing::info!("listening on {}", cfg.listen);

    while let Some(new_conn) = endpoint.accept().await {
        let root = cfg.root.clone();

        tokio::spawn(async move {
            let conn = match new_conn.await {
                Ok(c) => c,
                Err(e) => {
                    error!("connection failed: {}", e);
                    return;
                }
            };

            let mut h3_conn = h3::server::Connection::new(h3_quinn::Connection::new(conn))
                .await
                .unwrap();

            loop {
                match h3_conn.accept().await {
                    Ok(Some(resolver)) => {
                        let root = root.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_request(resolver, root).await {
                                error!("request failed: {}", e);
                            }
                        });
                    }
                    Ok(None) => break,
                    Err(e) => {
                        error!("accept error: {}", e);
                        break;
                    }
                }
            }
        });
    }

    endpoint.wait_idle().await;
    Ok(())
}

async fn handle_request<C>(
    resolver: RequestResolver<C, Bytes>,
    root: Arc<Option<PathBuf>>,
) -> Result<(), Box<dyn std::error::Error>>
where
    C: h3::quic::Connection<Bytes>,
{
    let (req, mut stream): (http::Request<()>, _) = resolver.resolve_request().await?;

    let (status, file) = match root.as_deref() {
        None => (StatusCode::OK, None),
        Some(_) if req.uri().path().contains("..") => (StatusCode::NOT_FOUND, None),
        Some(root) => {
            let path = root.join(req.uri().path().trim_start_matches('/'));
            match File::open(&path).await {
                Ok(f) => (StatusCode::OK, Some(f)),
                Err(_) => (StatusCode::NOT_FOUND, None),
            }
        }
    };

    let resp = http::Response::builder().status(status).body(())?;
    stream.send_response(resp).await?;

    if let Some(mut file) = file {
        loop {
            let mut buf = BytesMut::with_capacity(4096);
            if file.read_buf(&mut buf).await? == 0 {
                break;
            }
            stream.send_data(buf.freeze()).await?;
        }
    }

    stream.finish().await?;
    Ok(())
}
