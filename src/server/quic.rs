use crate::server::{AppConfig, h3, tls};
use quinn::{Endpoint, EndpointConfig};
use socket2::{Domain, Protocol, Socket, Type};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{error, info};

pub async fn run(cfg: Arc<AppConfig>) -> Result<(), Box<dyn std::error::Error>> {
    // Resolve host to IPv6 address
    let listen_addr: SocketAddr =
        if cfg.server.host.contains(':') && cfg.server.host.parse::<std::net::Ipv6Addr>().is_ok() {
            format!("[{}]:{}", cfg.server.host, cfg.server.port).parse()?
        } else {
            let mut addrs =
                tokio::net::lookup_host((cfg.server.host.as_str(), cfg.server.port)).await?;
            addrs.find(|a| a.is_ipv6()).ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::AddrNotAvailable,
                    format!("No IPv6 address found for host {}", cfg.server.host),
                )
            })?
        };

    info!("Starting IPv6 server on {}", listen_addr);

    // Load TLS config
    let tls_config = tls::load_tls_config(&cfg.server.cert_path, &cfg.server.key_path)?;
    let server_config = quinn::ServerConfig::with_crypto(Arc::new(tls_config));

    // Create IPv6-only UDP socket via socket2
    let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_only_v6(true)?;
    socket.bind(&listen_addr.into())?;
    let std_socket: std::net::UdpSocket = socket.into();

    let endpoint_config = EndpointConfig::default();
    let endpoint = Endpoint::new(
        endpoint_config,
        Some(server_config),
        std_socket,
        Arc::new(quinn::TokioRuntime),
    )?;

    info!("Server listening on {}", listen_addr);

    // Accept quic
    while let Some(incoming) = endpoint.accept().await {
        let cfg_clone = cfg.clone();

        tokio::spawn(async move {
            match incoming.await {
                Ok(conn) => {
                    if let Err(e) = h3::handle_connection(conn, cfg_clone).await {
                        error!("Connection error: {}", e);
                    }
                }
                Err(e) => error!("Failed to accept incoming connection: {}", e),
            }
        });
    }

    endpoint.wait_idle().await;
    Ok(())
}
