mod error;
mod health;
mod runtime;
mod servers;

use std::sync::Arc;

use app_base::{
    App, AppError,
    app::{Context, Privilege},
};
use tracing::info;

use crate::{cli::Cli, config::AppConfig, logging};
use error::AppRunError;

pub struct MotMot;

impl App for MotMot {
    type Config = AppConfig;
    type Cli = Cli;

    fn privilege() -> Privilege {
        Privilege::Root
    }

    fn run(&self, mut ctx: Context<Self::Config, Self::Cli>) -> Result<(), AppError> {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .expect("failed to install aws-lc-rs crypto provider");

        let (rt, local) = runtime::build_runtime()?;

        rt.block_on(local.run_until(async move {
            ctx.signals.install();

            loop {
                let config = Arc::new(ctx.config.clone());

                logging::init_logging_async(&config.logging)
                    .await
                    .map_err(|e| AppRunError::LoggingInit(format!("logging_init_failed: {e}")))?;

                info!("logging_initialized");

                health::run(&config).await?;

                let handles = servers::start_servers(config.clone(), ctx.signals.clone()).await;

                tokio::select! {
                    _ = ctx.signals.wait_shutdown() => {
                        info!("shutdown_signal_received");
                        servers::wait_servers(handles).await;
                        info!("shutdown_complete");
                        break;
                    }
                    _ = ctx.signals.wait_reload() => {
                        info!("reload_signal_received");

                        if let Err(e) = ctx.reload_config() {
                            tracing::error!(error = %e, "config_reload_failed");
                            continue;
                        }

                        info!("config_reloaded, restarting servers");
                        servers::wait_servers(handles).await;
                        info!("reload_complete");
                    }
                }
            }

            Ok(())
        }))
    }
}
