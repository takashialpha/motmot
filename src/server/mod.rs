pub mod fs;
pub mod h3;
pub mod quic;
pub mod request;
pub mod tls;

use std::sync::Arc;

pub use crate::config::AppConfig;

pub async fn run_server(cfg: Arc<AppConfig>) -> Result<(), Box<dyn std::error::Error>> {
    quic::run(cfg).await
}
