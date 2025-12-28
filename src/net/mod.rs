use std::{net::SocketAddr, sync::Arc};

use app_base::SignalHandler;
use quinn::{Endpoint, EndpointConfig};
use socket2::{Domain, Protocol, Socket, Type};
use tracing::{debug, error, info};

use crate::config::AppConfig;

pub mod h3;
pub mod quic;
pub mod tls;
pub mod webtransport;

use crate::net::quic::{ConnectionError, accept_loop};

pub async fn run_server(
    config: Arc<AppConfig>,
    server_name: String,
    signals: SignalHandler,
) -> Result<(), ConnectionError> {
    let server_config = config
        .servers
        .get(&server_name)
        .ok_or_else(|| ConnectionError::ServerNotFound(server_name.clone()))?;

    info!(
        server = %server_name,
        host = %server_config.host,
        port = server_config.port,
        webtransport = server_config.webtransport,
        "connection_setup_start"
    );

    let listen_addr = resolve_ipv6_addr(&server_config.host, server_config.port).await?;

    let tls_config = if let Some(tls_conf) = &server_config.tls {
        tls::load_or_generate(&server_name, Some(&tls_conf.cert), Some(&tls_conf.key)).await?
    } else {
        tls::load_or_generate(&server_name, None, None).await?
    };

    debug!(alpn = ?tls_config.alpn_protocols, "tls_alpn_configured");

    let endpoint = create_endpoint(&listen_addr, tls_config).await?;

    info!(server = %server_name, addr = %listen_addr, "connection_listening");

    // use unified accept loop
    let result =
        accept_loop::run_accept_loop(endpoint, Arc::clone(&config), server_name.clone(), signals)
            .await;

    match &result {
        Ok(_) => info!(server = %server_name, "connection_closed_clean"),
        Err(e) => error!(server = %server_name, error = %e, "connection_closed_error"),
    }

    result
}

/// resolve host to IPv6 address
async fn resolve_ipv6_addr(host: &str, port: u16) -> Result<SocketAddr, ConnectionError> {
    if host.contains(':')
        && let Ok(ipv6) = host.parse::<std::net::Ipv6Addr>()
    {
        return Ok(SocketAddr::from((ipv6, port)));
    }

    let mut addrs = tokio::net::lookup_host((host, port)).await.map_err(|e| {
        ConnectionError::AddressResolution {
            host: host.to_string(),
            source: e,
        }
    })?;

    addrs
        .find(|a| a.is_ipv6())
        .ok_or_else(|| ConnectionError::NoIpv6Address {
            host: host.to_string(),
        })
}

async fn create_endpoint(
    listen_addr: &SocketAddr,
    tls_config: rustls::ServerConfig,
) -> Result<Endpoint, ConnectionError> {
    let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))
        .map_err(ConnectionError::SocketCreation)?;

    socket
        .set_only_v6(true)
        .map_err(ConnectionError::SocketConfiguration)?;
    socket
        .bind(&(*listen_addr).into())
        .map_err(ConnectionError::SocketBind)?;

    let std_socket: std::net::UdpSocket = socket.into();

    let quic_server_config = quinn::crypto::rustls::QuicServerConfig::try_from(tls_config)
        .map_err(|e| {
            ConnectionError::EndpointCreation(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Failed to create QuicServerConfig: {}", e),
            ))
        })?;

    let server_config = quinn::ServerConfig::with_crypto(Arc::new(quic_server_config));
    let endpoint_config = EndpointConfig::default();

    Endpoint::new(
        endpoint_config,
        Some(server_config),
        std_socket,
        Arc::new(quinn::TokioRuntime),
    )
    .map_err(ConnectionError::EndpointCreation)
}
