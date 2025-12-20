use crate::server::error::ServerError;
use tokio::io::AsyncReadExt;
use tracing::debug;

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
    let (req, mut stream) = resolver.resolve_request().await?;

    debug!(
        server = %server_name,
        "Received request: {} {}",
        req.method(),
        req.uri().path()
    );

    let (status, file) =
        match crate::server::fs::resolve_file(req.uri().path(), config, server_name).await {
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

    debug!(
        server = %server_name,
        "Finished request: {} {}",
        req.method(),
        req.uri().path()
    );

    Ok(())
}
