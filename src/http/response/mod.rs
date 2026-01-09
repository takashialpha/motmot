pub mod error;
// mod

use bytes::Bytes;
use h3::server::RequestStream;
use http::{Response, StatusCode};

use crate::http::response::error::ResponseError;

pub async fn send(
    stream: &mut RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>,
    status: StatusCode,
    content_type: &str,
    body: &[u8],
) -> Result<(), ResponseError> {
    let response = Response::builder()
        .status(status)
        .header("content-type", content_type)
        .body(())
        .unwrap();

    stream.send_response(response).await?;
    stream.send_data(Bytes::copy_from_slice(body)).await?;
    stream.finish().await?;

    Ok(())
}
