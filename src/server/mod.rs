use std::sync::Arc;

use app_base::SignalHandler;
use quinn::Endpoint;
use tracing::{error, info};

use crate::config::AppConfig;

pub mod error;
pub mod request;

pub use error::ServerError;

pub async fn handle_connections(
    endpoint: Endpoint,
    config: Arc<AppConfig>,
    server_name: String,
    signals: SignalHandler,
) -> Result<(), ServerError> {
    info!(server = %server_name, "server_accept_loop_start");

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
                info!(server = %server_name, "server_shutdown_received");
                break;
            }
        }
    }

    info!(server = %server_name, "server_accept_loop_end");
    endpoint.wait_idle().await;

    Ok(())
}

async fn handle_connection(
    conn: quinn::Connection,
    config: Arc<AppConfig>,
    server_name: Arc<String>,
) -> Result<(), ServerError> {
    let mut h3_conn = h3::server::Connection::new(h3_quinn::Connection::new(conn))
        .await
        .map_err(ServerError::H3Connection)?;

    loop {
        match h3_conn.accept().await {
            Ok(Some(resolver)) => {
                let (req, stream) = match resolver.resolve_request().await {
                    Ok(resolved) => resolved,
                    Err(e) => {
                        error!(server = %server_name, error = %e, "request_resolve_failed");
                        continue;
                    }
                };

                let config = config.clone();
                let server_name = server_name.clone();

                tokio::spawn(async move {
                    let server_name_err = server_name.clone();
                    if let Err(e) = request::handle_request(req, stream, config, server_name).await
                    {
                        error!(
                            server = %server_name_err,
                            error = %e,
                            "request_handling_error"
                        );
                    }
                });
            }
            Ok(None) => break,
            Err(e) => {
                error!(
                    server = %server_name,
                    error = %e,
                    "connection_accept_request_error"
                );
                break;
            }
        }
    }

    Ok(())
}
