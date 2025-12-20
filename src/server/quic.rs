use crate::server::{AppConfig, error::ServerError, h3, tls};
use quinn::{Endpoint, EndpointConfig};
use socket2::{Domain, Protocol, Socket, Type};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{error, info};

pub async fn run(cfg: Arc<AppConfig>, server_name: String) -> Result<(), ServerError> {
    let server = cfg.servers.get(&server_name).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("server '{}' not found in config", server_name),
        )
    })?;

    // Resolve host to IPv6 address
    let listen_addr: SocketAddr =
        if server.host.contains(':') && server.host.parse::<std::net::Ipv6Addr>().is_ok() {
            format!("[{}]:{}", server.host, server.port).parse()?
        } else {
            let mut addrs = tokio::net::lookup_host((server.host.as_str(), server.port)).await?;
            addrs.find(|a| a.is_ipv6()).ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::AddrNotAvailable,
                    format!("No IPv6 address found for host {}", server.host),
                )
            })?
        };

    info!(server = %server_name, "Starting IPv6 server on {}", listen_addr);

    // Load TLS config (HTTP/3 + TLS 1.3 only)
    let tls_config = tls::load_tls_config(&server_name, &server.cert_path, &server.key_path)?;
    let server_config = quinn::ServerConfig::with_crypto(Arc::new(tls_config));

    // Create IPv6-only UDP socket
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

    info!(server = %server_name, "Server listening on {}", listen_addr);

    // Share server name cheaply across tasks
    let server_name = Arc::new(server_name);

    // Accept QUIC connections
    while let Some(incoming) = endpoint.accept().await {
        let cfg_clone = cfg.clone();
        let server_name = Arc::clone(&server_name);

        tokio::spawn(async move {
            match incoming.await {
                Ok(conn) => {
                    if let Err(e) =
                        h3::handle_connection(conn, cfg_clone, server_name.as_ref().clone()).await
                    {
                        error!(server = %server_name, error = %e, "Connection error");
                    }
                }
                Err(e) => {
                    error!(server = %server_name, error = %e, "Failed to accept incoming connection");
                }
            }
        });
    }

    endpoint.wait_idle().await;
    Ok(())
}
