use crate::server::{config::ServerConfig, h3, tls};
use h3_quinn::quinn;
use std::sync::Arc;
use tracing::{error, info};

pub async fn run(cfg: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting server on {}", cfg.listen);

    let tls_config = tls::load_tls_config(&cfg.cert_path, &cfg.key_path)?;
    let server_config = quinn::ServerConfig::with_crypto(Arc::new(tls_config));
    let endpoint = quinn::Endpoint::server(server_config, cfg.listen)?;

    info!("Server listening on {}", cfg.listen);

    while let Some(incoming) = endpoint.accept().await {
        let root = cfg.root.clone();

        tokio::spawn(async move {
            match incoming.await {
                Ok(conn) => {
                    if let Err(e) = h3::handle_connection(conn, root).await {
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
