use h3::server::Connection;
use h3_quinn::Connection as H3QuinnConnection;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info};

use crate::server::request;

pub async fn handle_connection(
    conn: h3_quinn::quinn::Connection,
    root: Option<Arc<PathBuf>>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("New QUIC connection established");

    let mut h3_conn = Connection::new(H3QuinnConnection::new(conn)).await?;
    info!("HTTP/3 connection initialized");

    while let Ok(Some(resolver)) = h3_conn.accept().await {
        let root = root.clone();
        tokio::spawn(async move {
            if let Err(e) = request::handle_request(resolver, root).await {
                error!("Request handling error: {}", e);
            }
        });
    }

    Ok(())
}
