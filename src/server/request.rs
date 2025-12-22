use crate::server::error::ServerError;
use std::time::Instant;
use tokio::io::AsyncReadExt;
use tracing::info;

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
    let start = Instant::now();

    let (req, mut stream) = resolver.resolve_request().await?;

    let path = req.uri().path();
    let method = req.method().clone();

    let (status, file) = match crate::server::fs::resolve_file(path, config, server_name).await {
        Ok(opt) => opt,
        Err(status) => (status, None),
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

    info!(
        server = %server_name,
        method = %method,
        path = %path,
        status = status.as_u16(),
        dur_ms = start.elapsed().as_millis(),
        "request"
    );

    Ok(())
}
