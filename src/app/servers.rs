use crate::net::quic::ConnectionError;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{Instrument, error, info};

use app_base::signals::SignalHandler;

use crate::config::AppConfig;
use crate::net::run_server;

pub async fn start_servers(
    config: Arc<AppConfig>,
    signals: SignalHandler,
) -> Vec<(String, JoinHandle<Result<(), ConnectionError>>)> {
    let mut handles = Vec::new();

    for (name, server_config) in &config.servers {
        info!(
            server = %name,
            host = %server_config.host,
            port = server_config.port,
            webtransport = server_config.webtransport,
            routes = server_config.routes.len(),
            "server_starting"
        );

        // clones for each cycle
        let server_name = name.clone();
        let server_name_for_task = server_name.clone();
        let config_for_task = config.clone();
        let signals_for_task = signals.clone();

        let span = tracing::info_span!("server", server = %server_name);

        let handle = tokio::spawn(
            async move {
                run_server(
                    config_for_task,
                    server_name_for_task,
                    signals_for_task,
                ).await
            }
            .instrument(span),
        );

        handles.push((server_name, handle));
    }

    info!(servers = handles.len(), "all_servers_started");
    handles
}

pub async fn wait_servers(handles: Vec<(String, JoinHandle<Result<(), ConnectionError>>)>) {
    for (name, handle) in handles {
        match handle.await {
            Ok(Ok(())) => info!(server = %name, "server_exited"),
            Ok(Err(e)) => error!(server = %name, error = %e, "server_error"),
            Err(e) => error!(server = %name, error = %e, "server_panic"),
        }
    }
}
