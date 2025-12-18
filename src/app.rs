use crate::{
    config::AppConfig,
    logging,
    server::{ServerConfig, run_server},
};
use app_base::{
    App,
    app::Context,
    error::{AppError, ConfigError},
};
use tokio::runtime::Runtime;

pub struct MotMot;

impl App for MotMot {
    type Config = AppConfig;

    fn run(&self, ctx: Context<Self::Config>) -> Result<(), AppError> {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .expect("failed to install aws-lc-rs crypto provider");

        let server_cfg = ServerConfig::from_app_config(&ctx.config.server)
            .map_err(|e| AppError::from(ConfigError::Io(e)))?;

        let rt = Runtime::new().map_err(|e| {
            AppError::from(ConfigError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e,
            )))
        })?;
        rt.block_on(async move {
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
        })
    }
}
