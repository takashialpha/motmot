use crate::server::error::ServerError;
use tokio::io::AsyncReadExt;

use h3::server::RequestResolver;

use crate::config::AppConfig;

pub async fn handle_request<C>(
    resolver: RequestResolver<C, bytes::Bytes>,
    config: &AppConfig,
    server_name: &str,
) -> Result<(), ServerError>
where
    C: h3::quic::Connection<bytes::Bytes>,
{
    let start = std::time::Instant::now();
    let (req, mut stream) = resolver.resolve_request().await?;
    let method = req.method().clone();
    let path = req.uri().path();

    // Handle all methods
    let (status, file) = match method {
        http::Method::GET | http::Method::HEAD => {
            match crate::server::fs::resolve_file(path, config, server_name).await {
                Ok(res) => res,
                Err(status) => (status, None),
            }
        }
        http::Method::POST
        | http::Method::PUT
        | http::Method::PATCH
        | http::Method::DELETE
        | http::Method::OPTIONS => (http::StatusCode::OK, None),
        _ => (http::StatusCode::METHOD_NOT_ALLOWED, None),
    };

    let resp = http::Response::builder().status(status).body(())?;
    stream.send_response(resp).await?;

    if let Some(mut file) = file {
        loop {
            let mut buf = bytes::BytesMut::with_capacity(4096 * 10);
            let n = file.read_buf(&mut buf).await?;
            if n == 0 {
                break;
            }
            stream.send_data(buf.freeze()).await?;
        }
    }

    stream.finish().await?;

    tracing::info!(
        server = %server_name,
        method = %method,
        path = %path,
        status = status.as_u16(),
        dur_ms = start.elapsed().as_millis(),
        "request_handled"
    );

    Ok(())
}
