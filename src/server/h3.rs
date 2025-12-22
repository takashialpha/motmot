use h3::server::Connection;
use std::sync::Arc;
use tracing::{Instrument, debug, error, info};

use crate::{config::AppConfig, server::error::ServerError};

pub async fn handle_connection(
    conn: h3_quinn::quinn::Connection,
    config: Arc<AppConfig>,
    server_name: String,
) -> Result<(), ServerError> {
    let remote = conn.remote_address();

    let conn_span = tracing::info_span!(
        "conn",
        server = %server_name,
        remote = %remote
    );

    async move {
        info!(
            server = %server_name,
            remote = %remote,
            "conn_open"
        );

        let mut h3_conn = Connection::new(h3_quinn::Connection::new(conn)).await?;

        loop {
            match h3_conn.accept().await {
                Ok(Some(resolver)) => {
                    let cfg = config.clone();
                    let server_name = server_name.clone();

                    tokio::spawn(
                        async move {
                            if let Err(e) =
                                crate::server::request::handle_request(resolver, &cfg, &server_name)
                                    .await
                            {
                                error!(
                                    server = %server_name,
                                    error = %e,
                                    "request_failed"
                                );
                            }
                        }
                        .instrument(tracing::Span::current()),
                    );
                }

                Ok(None) => break,

                Err(e) => {
                    // NOTE: ApplicationClose(0) is a normal client shutdown.
                    // NOTE: there are more normal client shutdowns. add them below!!!
                    let msg = e.to_string();

                    if msg.contains("ApplicationClose: 0x0") {
                        debug!(
                            server = %server_name,
                            remote = %remote,
                            error = %e,
                            "conn_remote_close"
                        );
                    } else {
                        error!(
                            server = %server_name,
                            remote = %remote,
                            error = %e,
                            "conn_h3_error"
                        );
                    }
                    break;
                }
            }
        }

        info!(
            server = %server_name,
            remote = %remote,
            "conn_close"
        );

        Ok(())
    }
    .instrument(conn_span)
    .await
}
