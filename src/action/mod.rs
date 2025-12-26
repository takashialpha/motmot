use std::sync::Arc;

use bytes::Bytes;
use http::{Response, StatusCode};

use crate::config::{Action, AppConfig};

pub mod error;
pub mod response;
pub mod static_file;

pub use error::ActionError;

/// Execute an action and return HTTP response
pub async fn execute(
    action: &Action,
    request_path: &str,
    config: Arc<AppConfig>,
    server_name: &str,
) -> Result<(Response<()>, Option<Bytes>), ActionError> {
    match action {
        Action::Static {
            directory,
            file,
            cache,
        } => static_file::serve(directory, file, *cache, request_path, server_name).await,

        Action::Proxy { .. } => {
            // Intentionally blocked until proxy abstraction is finalized
            Ok((
                Response::builder()
                    .status(StatusCode::NOT_IMPLEMENTED)
                    .body(())
                    .unwrap(),
                Some(Bytes::from_static(b"Proxy action not yet implemented")),
            ))
        }

        Action::Json { body, status } => response::json(body, *status),

        Action::Text {
            body,
            content_type,
            status,
        } => response::text(body, content_type, *status),

        Action::Redirect { to, status } => response::redirect(to, *status),

        Action::Deny { status, message } => response::deny(*status, message.as_deref()),
    }
}
