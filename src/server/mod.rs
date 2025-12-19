pub mod config;
pub mod fs;
pub mod h3;
pub mod quic;
pub mod request;
pub mod tls;

pub use config::ServerConfig;

pub async fn run_server(cfg: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    quic::run(cfg).await
}
