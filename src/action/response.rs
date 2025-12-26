use bytes::Bytes;
use http::{Response, StatusCode};

use super::error::ActionError;

/// Build a JSON response
pub fn json(body: &str, status: u16) -> Result<(Response<()>, Option<Bytes>), ActionError> {
    // Validate JSON
    if let Err(e) = serde_json::from_str::<serde_json::Value>(body) {
        return Err(ActionError::InvalidJson(format!(
            "Invalid JSON in config: {e}"
        )));
    }

    let status_code =
        StatusCode::from_u16(status).map_err(|e| ActionError::InvalidResponse(e.to_string()))?;

    let response = Response::builder()
        .status(status_code)
        .header("content-type", "application/json")
        .header("content-length", body.len().to_string())
        .body(())
        .map_err(|e| ActionError::InvalidResponse(e.to_string()))?;

    Ok((response, Some(Bytes::from(body.to_string()))))
}

/// Build a text response
pub fn text(
    body: &str,
    content_type: &str,
    status: u16,
) -> Result<(Response<()>, Option<Bytes>), ActionError> {
    let status_code =
        StatusCode::from_u16(status).map_err(|e| ActionError::InvalidResponse(e.to_string()))?;

    let response = Response::builder()
        .status(status_code)
        .header("content-type", content_type)
        .header("content-length", body.len().to_string())
        .body(())
        .map_err(|e| ActionError::InvalidResponse(e.to_string()))?;

    Ok((response, Some(Bytes::from(body.to_string()))))
}

/// Build a redirect response
pub fn redirect(to: &str, status: u16) -> Result<(Response<()>, Option<Bytes>), ActionError> {
    // Validate status code is a redirect (3xx)
    if !(300..400).contains(&status) {
        return Err(ActionError::InvalidResponse(format!(
            "Redirect status must be 3xx, got {status}"
        )));
    }

    let status_code =
        StatusCode::from_u16(status).map_err(|e| ActionError::InvalidResponse(e.to_string()))?;

    let response = Response::builder()
        .status(status_code)
        .header("location", to)
        .body(())
        .map_err(|e| ActionError::InvalidResponse(e.to_string()))?;

    // No body for redirects
    Ok((response, None))
}

/// Build a deny response
pub fn deny(
    status: u16,
    message: Option<&str>,
) -> Result<(Response<()>, Option<Bytes>), ActionError> {
    let status_code =
        StatusCode::from_u16(status).map_err(|e| ActionError::InvalidResponse(e.to_string()))?;

    let body = message.map(|m| Bytes::from(m.to_string()));
    let content_length = body.as_ref().map(|b| b.len()).unwrap_or(0);

    let mut response_builder = Response::builder().status(status_code);

    if body.is_some() {
        response_builder = response_builder
            .header("content-type", "text/plain; charset=utf-8")
            .header("content-length", content_length.to_string());
    }

    let response = response_builder
        .body(())
        .map_err(|e| ActionError::InvalidResponse(e.to_string()))?;

    Ok((response, body))
}
