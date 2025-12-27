use crate::config::AppConfig;
use bytes::Bytes;
use h3::server::RequestStream;
use std::sync::Arc;

pub async fn handle_request(
    _req: http::Request<()>,
    _stream: RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>,
    _config: Arc<AppConfig>,
    _server_name: Arc<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    todo!();
}
