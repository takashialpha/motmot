use std::sync::Arc;

use app_base::SignalHandler;
use h3::ext::Protocol;
use h3_webtransport::server::WebTransportSession;
use http::Method;
use quinn::Endpoint;
use tracing::{error, info};

use crate::config::AppConfig;

pub mod error;
pub mod session;

pub use error::WebTransportError;

pub async fn handle_connections(
    endpoint: Endpoint,
    config: Arc<AppConfig>,
    server_name: String,
    signals: SignalHandler,
) -> Result<(), WebTransportError> {
    info!(server = %server_name, "webtransport_accept_loop_start");

    let server_name = Arc::new(server_name);

    loop {
        tokio::select! {
            incoming = endpoint.accept() => {
                if let Some(incoming) = incoming {
                    let config = config.clone();
                    let server_name = server_name.clone();

                    tokio::spawn(async move {
                        match incoming.await {
                            Ok(conn) => {
                                let remote = conn.remote_address();
                                info!(
                                    server = %server_name,
                                    remote = %remote,
                                    "connection_established"
                                );

                                let server_name_err = server_name.clone();
                                if let Err(e) = handle_connection(conn, config, server_name).await {
                                    error!(
                                        server = %server_name_err,
                                        remote = %remote,
                                        error = %e,
                                        "connection_error"
                                    );
                                }
                            }
                            Err(e) => {
                                error!(
                                    server = %server_name,
                                    error = %e,
                                    "connection_accept_failed"
                                );
                            }
                        }
                    });
                }
            }
            _ = signals.wait_shutdown() => {
                info!(server = %server_name, "webtransport_shutdown_received");
                break;
            }
        }
    }

    info!(server = %server_name, "webtransport_accept_loop_end");
    endpoint.wait_idle().await;

    Ok(())
}

async fn handle_connection(
    conn: quinn::Connection,
    config: Arc<AppConfig>,
    server_name: Arc<String>,
) -> Result<(), WebTransportError> {
    let mut h3_conn = h3::server::builder()
        .enable_webtransport(true)
        .enable_extended_connect(true)
        .enable_datagram(true)
        .max_webtransport_sessions(1)
        .send_grease(true)
        .build(h3_quinn::Connection::new(conn))
        .await
        .map_err(WebTransportError::H3Connection)?;

    info!(
        server = %server_name,
        "h3_connection_established_with_webtransport"
    );

    loop {
        match h3_conn.accept().await {
            Ok(Some(resolver)) => {
                let (req, stream) = match resolver.resolve_request().await {
                    Ok(request) => request,
                    Err(e) => {
                        error!(
                            server = %server_name,
                            error = %e,
                            "request_resolve_failed"
                        );
                        continue;
                    }
                };

                let method = req.method();
                let path = req.uri().path().to_string();
                let ext = req.extensions();

                if method == Method::CONNECT
                    && ext.get::<Protocol>() == Some(&Protocol::WEB_TRANSPORT)
                {
                    info!(
                        server = %server_name,
                        path = %path,
                        "webtransport_session_requested"
                    );

                    let wt_session = WebTransportSession::accept(req, stream, h3_conn)
                        .await
                        .map_err(|e| WebTransportError::Session(e.to_string()))?;

                    info!(
                        server = %server_name,
                        session_id = ?wt_session.session_id(),
                        "webtransport_session_established"
                    );

                    let server_name_err = server_name.clone();
                    if let Err(e) =
                        session::handle_session(wt_session, config.clone(), server_name.clone())
                            .await
                    {
                        error!(
                            server = %server_name_err,
                            error = %e,
                            "webtransport_session_error"
                        );
                    }

                    return Ok(());
                } else {
                    info!(
                        server = %server_name,
                        method = %method,
                        path = %path,
                        "http3_request_received"
                    );

                    let config = config.clone();
                    let server_name = server_name.clone();

                    tokio::spawn(async move {
                        if let Err(e) =
                            crate::server::request::handle_request(req, stream, config, server_name)
                                .await
                        {
                            error!("http3_request_error: {}", e);
                        }
                    });
                }
            }
            Ok(None) => {
                info!(server = %server_name, "connection_closed");
                break;
            }
            Err(e) => {
                error!(
                    server = %server_name,
                    error = %e,
                    "connection_accept_error"
                );
                break;
            }
        }
    }

    Ok(())
}
