mod error;

use std::sync::Arc;

use h3::ext::Protocol;
use h3_quinn::Connection as H3QuinnConnection;
use h3_webtransport::server::WebTransportSession;
use http::Method;
use quinn::Connection;
use tracing::{debug, error, info};

use crate::config::AppConfig;
use crate::http::request;
use crate::net::h3::error::ServerError;

// forward to webtransport if CONNECT, handle conn accept loop and forward to request if not wt.
pub async fn handle_connection(
    conn: Connection,
    config: Arc<AppConfig>,
    server_name: Arc<String>,
) -> Result<(), ServerError> {
    let server_config = config
        .servers
        .get(&*server_name)
        .ok_or_else(|| ServerError::MissingServerConfig(server_name.to_string()))?;

    let mut builder = h3::server::builder();
    let builder = builder.enable_extended_connect(true);
    let builder = builder.enable_datagram(true);
    let mut h3_builder = builder.send_grease(true);

    if server_config.webtransport {
        h3_builder = h3_builder
            .enable_webtransport(true)
            .max_webtransport_sessions(1);
    }

    let mut h3_conn = h3_builder.build(H3QuinnConnection::new(conn)).await?;

    loop {
        match h3_conn.accept().await {
            Ok(Some(resolver)) => {
                let (req, stream) = match resolver.resolve_request().await {
                    Ok(r) => r,
                    Err(e) => {
                        error!(server = %server_name, error = %e, "request_resolve_failed");
                        continue;
                    }
                };

                let method = req.method();
                let ext = req.extensions();
                let path = req.uri().path().to_string();

                if server_config.webtransport
                    && method == Method::CONNECT
                    && ext.get::<Protocol>() == Some(&Protocol::WEB_TRANSPORT)
                {
                    info!(server = %server_name, path = %path, "webtransport_session_requested");

                    let wt_session = WebTransportSession::accept(req, stream, h3_conn)
                        .await
                        .map_err(|e| ServerError::WebTransport(e.to_string()))?;

                    info!(
                        server = %server_name,
                        session_id = ?wt_session.session_id(),
                        "webtransport_session_established"
                    );

                    if let Err(e) = crate::net::webtransport::handle_session(
                        wt_session,
                        config.clone(),
                        server_name.clone(),
                    )
                    .await
                    {
                        error!(server = %server_name, error = %e, "webtransport_session_error");
                    }

                    return Ok(());
                }

                let config_clone = config.clone();
                let server_name_clone = server_name.clone();
                tokio::spawn(async move {
                    if let Err(e) = request::handle_request(
                        req,
                        stream,
                        config_clone,
                        server_name_clone.clone(),
                    )
                    .await
                    {
                        error!(server = %server_name_clone, error = %e, "http3_request_error");
                    }
                });
            }
            Ok(None) => {
                info!(server = %server_name, "connection_closed");
                break;
            }
            Err(e) => {
                debug!(server = %server_name, error = %e);
                return Err(e.into());
            }
        }
    }

    Ok(())
}
