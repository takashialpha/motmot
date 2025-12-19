use bytes::{Bytes, BytesMut};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tracing::debug;

use h3::server::RequestResolver;

use crate::server::fs;

pub async fn handle_request<C>(
    resolver: RequestResolver<C, Bytes>,
    root: Option<Arc<PathBuf>>,
) -> Result<(), Box<dyn std::error::Error>>
where
    C: h3::quic::Connection<Bytes>,
{
    let (req, mut stream): (http::Request<()>, _) = resolver.resolve_request().await?;
    debug!("Received request: {} {}", req.method(), req.uri().path());

    let (status, file) = match fs::determine_file(&req, root.as_deref()).await {
        Ok(opt) => opt,
        Err(status) => (status, None),
    };

    let resp = http::Response::builder().status(status).body(())?;
    stream.send_response(resp).await?;

    if let Some(mut file) = file {
        loop {
            let mut buf = BytesMut::with_capacity(4096 * 10);
            let n = file.read_buf(&mut buf).await?;
            if n == 0 {
                break;
            }
            stream.send_data(buf.freeze()).await?;
        }
    }

    stream.finish().await?;
    debug!("Finished request: {} {}", req.method(), req.uri().path());
    Ok(())
}
