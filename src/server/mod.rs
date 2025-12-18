use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use bytes::{Bytes, BytesMut};
use http::StatusCode;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio::{fs::File, io::AsyncReadExt};
use tracing::{debug, error, info};

use h3::server::RequestResolver;
use h3_quinn::quinn::{self, crypto::rustls::QuicServerConfig};

static ALPN: &[u8] = b"h3";

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub root: Option<Arc<PathBuf>>,
    pub listen: SocketAddr,
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}

impl ServerConfig {
    pub fn from_app_config(server: &crate::config::Server) -> std::io::Result<Self> {
        let listen = format!("{}:{}", server.host, server.port)
            .parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

        let root = server.root.as_ref().map(|p| Arc::new(p.clone()));

        Ok(Self {
            root,
            listen,
            cert_path: server.cert_path.clone(),
            key_path: server.key_path.clone(),
        })
    }
}

pub async fn run_server(cfg: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting server on {}", cfg.listen);

    let tls_config = load_tls_config(&cfg.cert_path, &cfg.key_path)?;
    let server_config = quinn::ServerConfig::with_crypto(Arc::new(tls_config));
    let endpoint = quinn::Endpoint::server(server_config, cfg.listen)?;

    info!("Server listening on {}", cfg.listen);

    // Accept incoming connections
    while let Some(incoming) = endpoint.accept().await {
        let root = cfg.root.clone();

        tokio::spawn(async move {
            match incoming.await {
                Ok(conn) => {
                    if let Err(e) = handle_connection(conn, root).await {
                        error!("Connection error: {}", e);
                    }
                }
                Err(e) => error!("Failed to accept incoming connection: {}", e),
            }
        });
    }

    endpoint.wait_idle().await;
    info!("Server has shut down");
    Ok(())
}

/// Load TLS certificates and keys
fn load_tls_config(
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

/// Handle a single QUIC connection
async fn handle_connection(
    conn: quinn::Connection,
    root: Option<Arc<PathBuf>>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("New QUIC connection established");

    let mut h3_conn = h3::server::Connection::new(h3_quinn::Connection::new(conn)).await?;
    info!("HTTP/3 connection initialized");

    while let Ok(Some(resolver)) = h3_conn.accept().await {
        let root = root.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_request(resolver, root).await {
                error!("Request handling error: {}", e);
            }
        });
    }

    Ok(())
}

/// Handle a single HTTP/3 request
async fn handle_request<C>(
    resolver: RequestResolver<C, Bytes>,
    root: Option<Arc<PathBuf>>,
) -> Result<(), Box<dyn std::error::Error>>
where
    C: h3::quic::Connection<Bytes>,
{
    let (req, mut stream): (http::Request<()>, _) = resolver.resolve_request().await?;
    debug!("Received request: {} {}", req.method(), req.uri().path());

    let (status, file) = match determine_file(&req, root.as_deref()).await {
        Ok(opt) => opt,
        Err(status) => (status, None),
    };

    // Send response headers
    let resp = http::Response::builder().status(status).body(())?;
    stream.send_response(resp).await?;

    // Stream file if available
    if let Some(mut file) = file {
        loop {
            let mut buf = BytesMut::with_capacity(4096 * 10);
            let n = file.read_buf(&mut buf).await?;
            if n == 0 {
                break;
            }
            stream.send_data(buf.freeze()).await?;
        }
    }

    stream.finish().await?;
    debug!("Finished request: {} {}", req.method(), req.uri().path());
    Ok(())
}

/// Determine which file to serve (async)
async fn determine_file(
    req: &http::Request<()>,
    root: Option<&PathBuf>,
) -> Result<(StatusCode, Option<File>), StatusCode> {
    match root {
        None => Ok((StatusCode::OK, None)),
        Some(root) if req.uri().path().contains("..") => Err(StatusCode::NOT_FOUND),
        Some(root) => {
            let path = root.join(req.uri().path().trim_start_matches('/'));
            match File::open(&path).await {
                Ok(f) => Ok((StatusCode::OK, Some(f))),
                Err(_) => Err(StatusCode::NOT_FOUND),
            }
        }
    }
}
