use app_base::SignalHandler;
use quinn::Endpoint;
use std::sync::Arc;
use tracing::{error, info};

use super::error::ConnectionError;
use crate::config::AppConfig;
use crate::net::h3;

// unified accept loop for http3 (also calls wt handler)
pub async fn run_accept_loop(
    endpoint: Endpoint,
    config: Arc<AppConfig>,
    server_name: String,
    signals: SignalHandler,
) -> Result<(), ConnectionError> {
    let server_name = Arc::new(server_name);
    info!(server = %server_name, "accept_loop_start");

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
                                info!(server = %server_name, remote = %remote, "connection_established");

                                if let Err(e) = h3::handle_connection(conn, config.clone(), server_name.clone()).await {
                                    error!(server = %server_name, remote = %remote, error = %e, "connection_error");
                                }
                            }
                            Err(e) => error!(server = %server_name, error = %e, "connection_accept_failed"),
                        }
                    });
                }
            }
            _ = signals.wait_shutdown() => {
                info!(server = %server_name, "shutdown_received");
                break;
            }
        }
    }

    info!(server = %server_name, "accept_loop_end");
    endpoint.wait_idle().await;
    Ok(())
}
