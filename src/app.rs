use std::sync::Arc;

use app_base::{
    App,
    app::{Context, Privilege},
    error::{AppError, ConfigError},
};
use tokio::{runtime::Runtime, task};
use tracing::{error, info};

use crate::net::run_server;
use crate::{config::AppConfig, logging};

pub struct MotMot;

impl App for MotMot {
    type Config = AppConfig;

    fn privilege() -> Privilege {
        Privilege::Root
    }

    fn run(&self, mut ctx: Context<Self::Config>) -> Result<(), AppError> {
        // Install crypto provider (required for rustls)
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .expect("failed to install aws-lc-rs crypto provider");

        let rt = Runtime::new()
            .map_err(|e| AppError::from(ConfigError::Io(std::io::Error::other(e))))?;

        let local = task::LocalSet::new();

        rt.block_on(local.run_until(async move {
            ctx.signals.install(); // install signals here

            loop {
                let config = Arc::new(ctx.config.clone());

                logging::init_logging_async(&config.logging)
                    .await
                    .map_err(|e| {
                        AppError::from(ConfigError::Io(std::io::Error::other(format!(
                            "logging_init_failed: {e}"
                        ))))
                    })?;

                info!("logging_initialized");

                #[cfg(feature = "health")]
                {
                    if config.health.enabled {
                        info!("health_check_starting");
                        crate::features::health::run_checks(&config)
                            .await
                            .map_err(|e| {
                                error!("health_check_failed: {e}");
                                AppError::from(ConfigError::Io(std::io::Error::other(format!(
                                    "health_check_failed: {e}"
                                ))))
                            })?;
                        info!("health_check_passed");
                    } else {
                        info!("health_check_disabled: config");
                    }
                }

                #[cfg(not(feature = "health"))]
                {
                    info!("health_check_disabled: not built");
                }

                let mut handles = Vec::new();

                for (name, server_config) in &config.servers {
                    info!(
                        server = %name,
                        host = %server_config.host,
                        port = server_config.port,
                        webtransport = server_config.webtransport,
                        routes = server_config.routes.len(),
                        "server_starting"
                    );

                    let config_clone = config.clone();
                    let server_name = name.clone();
                    let server_name_clone = server_name.clone();
                    let signals = ctx.signals.clone();

                    let span = tracing::info_span!("server", server = %server_name);

                    let handle = tokio::spawn(
                        async move { run_server(config_clone, server_name_clone, signals).await }
                            .instrument(span),
                    );

                    handles.push((server_name.clone(), handle));
                }

                info!(servers = handles.len(), "all_servers_started");

                tokio::select! {
                    _ = ctx.signals.wait_shutdown() => {
                        info!("shutdown_signal_received");

                        for (name, handle) in handles {
                            match handle.await {
                                Ok(Ok(())) => info!(server = %name, "server_exited"),
                                Ok(Err(e)) => error!(server = %name, error = %e, "server_error"),
                                Err(e) => error!(server = %name, error = %e, "server_panic"),
                            }
                        }

                        info!("shutdown_complete");
                        break;
                    }
                    _ = ctx.signals.wait_reload() => {
                        info!("reload_signal_received");

                        match ctx.reload_config() {
                            Ok(()) => {
                                info!("config_reloaded, restarting servers");
                            }
                            Err(e) => {
                                error!(error = %e, "config_reload_failed");
                                continue; // by default not restarting nor gonna apply new config
                            }
                        }

                        for (name, handle) in handles {
                            match handle.await {
                                Ok(Ok(())) => info!(server = %name, "server_exited"),
                                Ok(Err(e)) => error!(server = %name, error = %e, "server_error"),
                                Err(e) => error!(server = %name, error = %e, "server_panic"),
                            }
                        }

                        info!("reload_complete");
                        continue;
                    }
                }
            }

            Ok(())
        }))
    }
}

use tracing::Instrument;
