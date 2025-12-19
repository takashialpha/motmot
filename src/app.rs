use crate::{config::AppConfig, logging, server::run_server};
use app_base::{
    App,
    app::{Context, Privilege},
    error::{AppError, ConfigError},
};

use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::task;

pub struct MotMot;

impl App for MotMot {
    type Config = AppConfig;

    fn privilege() -> Privilege {
        Privilege::Root
    }
    fn run(&self, ctx: Context<Self::Config>) -> Result<(), AppError> {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .expect("failed to install aws-lc-rs crypto provider");

        // let server_cfg = ServerConfig::from_app_config(&ctx.config.server)
        //     .map_err(|e| AppError::from(ConfigError::Io(e)))?;

        let server_cfg = Arc::new(ctx.config.clone());

        let rt = Runtime::new().map_err(|e| {
            AppError::from(ConfigError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e,
            )))
        })?;

        let local = task::LocalSet::new();

        rt.block_on(local.run_until(async move {
            logging::init_logging_async(&ctx.config.logging)
                .await
                .map_err(|e| {
                    AppError::from(ConfigError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to initialize logging: {e}"),
                    )))
                })?;

            run_server(server_cfg).await.map_err(|e| {
                AppError::from(ConfigError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("{e}"),
                )))
            })
        }))
    }
}
