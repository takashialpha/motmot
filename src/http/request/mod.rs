mod error;

use crate::config::{Action, AppConfig};
use crate::helpers::{fs as static_fs, mime};
use crate::http::request::error::RequestError;
use crate::http::response;
use bytes::Bytes;
use h3::server::RequestStream;
use http::StatusCode;
use std::sync::Arc;

pub async fn handle_request(
    req: http::Request<()>,
    mut stream: RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>,
    config: Arc<AppConfig>,
    server_name: Arc<String>,
) -> Result<(), RequestError> {
    let server = config
        .servers
        .get(&*server_name)
        .ok_or_else(|| RequestError::Config("missing server config".into()))?;

    let path = req.uri().path();
    let method = req.method().as_str();

    let route = match server.routes.get(path) {
        Some(route) => route,
        None => {
            return execute_action(&server.standard.not_found, &mut stream).await;
        }
    };

    let action = match route.methods.get(method) {
        Some(action) => action,
        None => {
            return execute_action(&server.standard.method_not_allowed, &mut stream).await;
        }
    };

    // execute resolved action
    if execute_action(action, &mut stream).await.is_err() {
        return execute_action(&server.standard.internal_error, &mut stream).await;
    }

    Ok(())
}

async fn execute_action(
    action: &Action,
    stream: &mut RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>,
) -> Result<(), RequestError> {
    match action {
        Action::Response {
            body,
            content_type,
            status,
        } => response::send(
            stream,
            StatusCode::from_u16(*status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            content_type,
            body.as_bytes(),
        )
        .await
        .map_err(Into::into),

        Action::Static { path, .. } => match static_fs::read(path).await {
            static_fs::StaticRead::Ok(data) => {
                let ct = mime::from_bytes(&data);
                response::send(stream, StatusCode::OK, ct, &data)
                    .await
                    .map_err(Into::into)
            }

            static_fs::StaticRead::NotFound => response::send(
                stream,
                StatusCode::NOT_FOUND,
                "text/plain; charset=utf-8",
                b"Not Found",
            )
            .await
            .map_err(Into::into),

            static_fs::StaticRead::Forbidden => response::send(
                stream,
                StatusCode::FORBIDDEN,
                "text/plain; charset=utf-8",
                b"Forbidden",
            )
            .await
            .map_err(Into::into),

            static_fs::StaticRead::Error => response::send(
                stream,
                StatusCode::INTERNAL_SERVER_ERROR,
                "text/plain; charset=utf-8",
                b"Internal Server Error",
            )
            .await
            .map_err(Into::into),
        },

        Action::Proxy { .. } => {
            // not implemented.
            response::send(
                stream,
                StatusCode::NOT_IMPLEMENTED,
                "text/plain; charset=utf-8",
                b"Proxy not implemented",
            )
            .await
            .map_err(Into::into)
        }

        Action::Script { .. } => {
            // not implemented.
            response::send(
                stream,
                StatusCode::NOT_IMPLEMENTED,
                "text/plain; charset=utf-8",
                b"Script execution not implemented",
            )
            .await
            .map_err(Into::into)
        }
    }
}
