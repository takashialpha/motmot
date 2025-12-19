use h3::server::Connection;
use std::sync::Arc;

use crate::config::AppConfig;

pub async fn handle_connection(
    conn: h3_quinn::quinn::Connection,
    config: Arc<AppConfig>,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("New QUIC connection established");

    let mut h3_conn = Connection::new(h3_quinn::Connection::new(conn)).await?;

    while let Ok(Some(resolver)) = h3_conn.accept().await {
        let cfg = config.clone();
        tokio::spawn(async move {
            if let Err(e) = crate::server::request::handle_request(resolver, &cfg).await {
                tracing::error!("Request handling error: {}", e);
            }
        });
    }

    Ok(())
}
