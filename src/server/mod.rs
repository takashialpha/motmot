pub mod error;
pub mod fs;
pub mod h3;
pub mod quic;
pub mod request;
pub mod tls;

pub use crate::config::AppConfig;
use crate::server::error::ServerError;

use std::sync::Arc;
use tracing::Instrument;

pub async fn run_server(cfg: Arc<AppConfig>) -> Result<(), ServerError> {
    let mut handles = Vec::new();

    for (name, _) in &cfg.servers {
        let cfg_clone = cfg.clone();

        let server_name = name.clone(); // for bookkeeping
        let task_name = server_name.clone(); // for the async task

        let span = tracing::info_span!("server", server = %server_name);

        let handle =
            tokio::spawn(async move { quic::run(cfg_clone, task_name).await }.instrument(span));

        handles.push((server_name, handle));
    }

    for (name, handle) in handles {
        match handle.await {
            Ok(Ok(())) => {
                tracing::info!(server = %name, "Server exited");
            }
            Ok(Err(e)) => {
                tracing::error!(server = %name, error = %e, "Server exited with error");
            }
            Err(e) => {
                tracing::error!(server = %name, error = %e, "Server task panicked");
            }
        }
    }

    Ok(())
}
