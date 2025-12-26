use std::time::Duration;

use bytes::Bytes;
use http::{HeaderMap, Method, Request, Response, StatusCode, Uri};
use tracing::{error, info, warn};

pub mod error;

pub use error::ProxyError;

/// Forward a request to an upstream HTTP server
///
/// Supports HTTP/1.1, HTTP/2, and HTTP/3 backends automatically.
/// Uses connection pooling for better performance.
pub async fn forward_request(
    upstream: &str,
    request_method: &Method,
    request_path: &str,
    request_headers: &HeaderMap,
    request_body: Option<Bytes>,
    preserve_host: bool,
    timeout_secs: u64,
    strip_prefix: Option<&str>,
    server_name: &str,
) -> Result<(Response<()>, Option<Bytes>), ProxyError> {
    let start = std::time::Instant::now();

    // Parse upstream URL
    let upstream_uri = upstream
        .parse::<Uri>()
        .map_err(|_| ProxyError::InvalidUpstreamUrl {
            url: upstream.to_string(),
        })?;

    // Build target path (with prefix stripping if configured)
    let target_path = if let Some(prefix) = strip_prefix {
        request_path.strip_prefix(prefix).unwrap_or(request_path)
    } else {
        request_path
    };

    // Build full upstream URL
    let target_url = format!(
        "{}://{}{}{}",
        upstream_uri.scheme_str().unwrap_or("http"),
        upstream_uri.authority().map(|a| a.as_str()).unwrap_or(""),
        target_path,
        upstream_uri
            .query()
            .map(|q| format!("?{}", q))
            .unwrap_or_default()
    );

    info!(
        server = server_name,
        upstream = upstream,
        target = %target_url,
        method = %request_method,
        "proxy_forward_start"
    );

    // Create HTTP client (supports HTTP/1.1, HTTP/2, HTTP/3)
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .pool_max_idle_per_host(10)
        .pool_idle_timeout(Duration::from_secs(90))
        .http2_prior_knowledge() // Enable HTTP/2 if available
        .build()
        .map_err(|e| ProxyError::UpstreamConnectionFailed {
            upstream: upstream.to_string(),
            source: e.to_string().into(),
        })?;

    // Build request
    let mut req_builder = client.request(request_method.clone(), &target_url);

    // Forward headers
    for (name, value) in request_headers.iter() {
        // Skip hop-by-hop headers
        if is_hop_by_hop_header(name.as_str()) {
            continue;
        }

        // Handle Host header based on preserve_host setting
        if name == "host" && !preserve_host {
            continue;
        }

        req_builder = req_builder.header(name.as_str(), value.as_bytes());
    }

    // Add body if present
    if let Some(body) = request_body {
        req_builder = req_builder.body(body);
    }

    // Send request with timeout
    let upstream_response = req_builder.send().await.map_err(|e| {
        if e.is_timeout() {
            ProxyError::UpstreamTimeout { timeout_secs }
        } else if e.is_connect() {
            ProxyError::UpstreamConnectionFailed {
                upstream: upstream.to_string(),
                source: e.to_string().into(),
            }
        } else {
            ProxyError::UpstreamError {
                status: e.status().map(|s| s.as_u16()).unwrap_or(500),
                message: e.to_string(),
            }
        }
    })?;

    let status = upstream_response.status();
    let upstream_headers = upstream_response.headers().clone();

    // Read response body
    let body_bytes = upstream_response
        .bytes()
        .await
        .map_err(|e| ProxyError::UpstreamError {
            status: status.as_u16(),
            message: e.to_string(),
        })?;

    // Build response
    let mut response_builder = Response::builder().status(status);

    // Forward response headers (skip hop-by-hop)
    for (name, value) in upstream_headers.iter() {
        if is_hop_by_hop_header(name.as_str()) {
            continue;
        }
        response_builder = response_builder.header(name.as_str(), value.as_bytes());
    }

    let response = response_builder
        .body(())
        .map_err(|e| ProxyError::ResponseBuildError(e.to_string()))?;

    let duration_ms = start.elapsed().as_millis();

    info!(
        server = server_name,
        upstream = upstream,
        target = %target_url,
        method = %request_method,
        status = status.as_u16(),
        bytes = body_bytes.len(),
        dur_ms = duration_ms,
        "proxy_forward_complete"
    );

    Ok((response, Some(body_bytes)))
}

/// Check if a header is hop-by-hop (should not be forwarded)
fn is_hop_by_hop_header(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailers"
            | "transfer-encoding"
            | "upgrade"
    )
}
