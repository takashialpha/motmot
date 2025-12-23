pub mod error;
pub mod fs;
pub mod h3;
pub mod quic;
pub mod request;
pub mod tls;

pub use crate::config::AppConfig;
use crate::server::error::ServerError;

use std::sync::Arc;
use tokio::sync::Notify;
use tracing::Instrument;

pub async fn run_server(cfg: Arc<AppConfig>) -> Result<(), ServerError> {
    let shutdown = Arc::new(Notify::new());

    // Spawn signal handler
    let shutdown_clone = shutdown.clone();
    tokio::spawn(async move {
        handle_signals(shutdown_clone).await;
    });

    let mut handles = Vec::new();

    for name in cfg.servers.keys() {
        let cfg_clone = cfg.clone();
        let server_name = name.clone();
        let server_name_clone = server_name.clone();
        let shutdown_clone = shutdown.clone();

        let span = tracing::info_span!("server", server = %server_name);

        let handle = tokio::spawn(
            async move { quic::run(cfg_clone, server_name_clone, shutdown_clone).await }
                .instrument(span),
        );

        handles.push((server_name, handle));
    }

    for (name, handle) in handles {
        match handle.await {
            Ok(Ok(())) => tracing::info!(server = %name, "Server exited"),
            Ok(Err(e)) => tracing::error!(server = %name, error = %e, "Server exited with error"),
            Err(e) => tracing::error!(server = %name, error = %e, "Server task panicked"),
        }
    }

    Ok(())
}

async fn handle_signals(shutdown: Arc<Notify>) {
    use tokio::signal::unix::{SignalKind, signal};

    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();

    tokio::select! {
        _ = sigint.recv() => tracing::info!("SIGINT received, shutting down..."),
        _ = sigterm.recv() => tracing::info!("SIGTERM received, shutting down..."),
    }

    shutdown.notify_waiters();
}
