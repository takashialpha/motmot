use crate::{config::AppConfig, server};
use app_base::{
    App,
    app::Context,
    error::{AppError, ConfigError},
};

use rustls::crypto::CryptoProvider;
use std::{net::SocketAddr, sync::Arc};

pub struct MotMot;

impl App for MotMot {
    type Config = AppConfig;

    fn run(&self, ctx: Context<Self::Config>) -> Result<(), AppError> {
        let server_cfg = &ctx.config.server;

        let addr: SocketAddr = format!("{}:{}", server_cfg.host, server_cfg.port)
            .parse()
            .map_err(|e| {
                AppError::from(ConfigError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    e,
                )))
            })?;

        let cert = std::fs::read(&server_cfg.cert_path)
            .map_err(ConfigError::Io)
            .map_err(AppError::from)?;

        let key = std::fs::read(&server_cfg.key_path)
            .map_err(ConfigError::Io)
            .map_err(AppError::from)?;

        let server_config = server::ServerConfig {
            root: Arc::new(server_cfg.root.clone()),
            listen: addr,
            cert: Arc::new(cert),
            key: Arc::new(key),
        };

        let rt = tokio::runtime::Runtime::new()
            .map_err(ConfigError::Io)
            .map_err(AppError::from)?;

        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .expect("failed to install aws-lc-rs crypto provider");

        rt.block_on(server::run_server(server_config))
            .map_err(|e| {
                AppError::from(ConfigError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))
            })?;

        Ok(())
    }
}
