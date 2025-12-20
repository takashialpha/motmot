use h3::server::Connection;
use std::sync::Arc;
use tracing::{error, info};

use crate::{config::AppConfig, server::error::ServerError};

pub async fn handle_connection(
    conn: h3_quinn::quinn::Connection,
    config: Arc<AppConfig>,
    server_name: String,
) -> Result<(), ServerError> {
    info!(server = %server_name, "New QUIC connection established");

    let mut h3_conn = Connection::new(h3_quinn::Connection::new(conn)).await?;

    while let Ok(Some(resolver)) = h3_conn.accept().await {
        let cfg = config.clone();
        let server_name = server_name.clone();

        tokio::spawn(async move {
            if let Err(e) =
                crate::server::request::handle_request(resolver, &cfg, &server_name).await
            {
                error!(
                    server = %server_name,
                    "Request handling error: {}", e
                );
            }
        });
    }

    Ok(())
}
