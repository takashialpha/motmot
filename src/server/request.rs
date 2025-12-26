use std::sync::Arc;

use bytes::Buf;
use h3::server::RequestStream;
use http::{Method, Request};
use tracing::info;

use crate::config::AppConfig;
use crate::tools;

use super::error::ServerError;

/// Handle a single HTTP/3 request
pub async fn handle_request<T>(
    req: Request<()>,
    mut stream: RequestStream<T, bytes::Bytes>,
    config: Arc<AppConfig>,
    server_name: Arc<String>,
) -> Result<(), ServerError>
where
    T: h3::quic::BidiStream<bytes::Bytes>,
{
    let start = std::time::Instant::now();
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let headers = req.headers().clone();

    // Read request body if present
    let mut body_bytes = bytes::BytesMut::new();
    while let Some(mut chunk) = stream.recv_data().await.map_err(ServerError::H3Stream)? {
        body_bytes.extend_from_slice(chunk.chunk());
        let remaining = chunk.remaining();
        chunk.advance(remaining);
    }

    let body = if body_bytes.is_empty() {
        None
    } else {
        Some(body_bytes.freeze())
    };

    // Get server config
    let server_config = config
        .servers
        .get(server_name.as_str())
        .ok_or_else(|| ServerError::InvalidRequest("server not found".to_string()))?;

    // Match route
    let (matched_route, route_config) = tools::match_route(&path, &server_config.routes)
        .ok_or_else(|| {
            ServerError::RouteNotFound(format!("no route matched for path: {}", path))
        })?;

    // Select action based on HTTP method
    let action = select_action_for_method(&method, route_config).ok_or_else(|| {
        ServerError::MethodNotAllowed(format!(
            "method {} not allowed for route {}",
            method, matched_route
        ))
    })?;

    // Execute action
    let (response, response_body) =
        crate::action::execute(action, &path, Arc::clone(&config), server_name.as_str())
            .await
            .map_err(|e| ServerError::ActionExecution(e.to_string()))?;

    let status = response.status();

    // Send response headers
    stream
        .send_response(response)
        .await
        .map_err(ServerError::H3Stream)?;

    // Send response body if present
    if let Some(body_bytes) = response_body {
        stream
            .send_data(body_bytes)
            .await
            .map_err(ServerError::H3Stream)?;
    }

    // Finish stream
    stream.finish().await.map_err(ServerError::H3Stream)?;

    let duration_ms = start.elapsed().as_millis();

    info!(
        server = %server_name,
        method = %method,
        path = %path,
        route = matched_route,
        status = status.as_u16(),
        dur_ms = duration_ms,
        "request_complete"
    );

    Ok(())
}

/// Select the appropriate action for an HTTP method
fn select_action_for_method<'a>(
    method: &Method,
    route_config: &'a crate::config::RouteConfig,
) -> Option<&'a crate::config::Action> {
    match *method {
        Method::GET => route_config.get.as_ref(),
        Method::POST => route_config.post.as_ref(),
        Method::PUT => route_config.put.as_ref(),
        Method::DELETE => route_config.delete.as_ref(),
        Method::PATCH => route_config.patch.as_ref(),
        Method::HEAD => route_config.head.as_ref().or(route_config.get.as_ref()),
        Method::OPTIONS => route_config.options.as_ref(),
        _ => None,
    }
    .or(route_config.fallback.as_ref())
}
