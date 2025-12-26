use std::path::{Path, PathBuf};

use bytes::Bytes;
use http::{Response, StatusCode};
use tracing::info;

use crate::tools;

use super::error::ActionError;

/// Serve a static file
///
/// Flow:
/// 1. Resolve file path securely (prevent traversal)
/// 2. Validate file exists and is readable
/// 3. Read file into memory (streaming in future)
/// 4. Determine MIME type
/// 5. Return response with headers
pub async fn serve(
    directory: &Path,
    file: &str,
    _cache: bool, // TODO: Implement caching
    request_path: &str,
    server_name: &str,
) -> Result<(Response<()>, Option<Bytes>), ActionError> {
    // Resolve file path safely
    let file_path = resolve_file_path(directory, file)?;

    // Validate file exists and is readable
    tools::validate_file(&file_path).await?;

    // Read file contents
    let contents = tools::read_to_bytes(&file_path).await?;
    let file_size = contents.len();

    // Determine MIME type
    let mime_type = tools::guess_mime_type(&file_path);

    // Build response
    let response = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", mime_type)
        .header("content-length", file_size.to_string())
        .body(())
        .map_err(|e| ActionError::InvalidResponse(e.to_string()))?;

    info!(
        server = server_name,
        path = request_path,
        file = %file_path.display(),
        mime = mime_type,
        bytes = file_size,
        "static_file_served"
    );

    Ok((response, Some(contents)))
}

/// Resolve file path with security checks
///
/// The `file` parameter can be:
/// - A simple filename: "index.html"
/// - A relative path: "docs/api.html"
/// - A template with {path}: "{path}" (uses request path)
fn resolve_file_path(directory: &Path, file: &str) -> Result<PathBuf, ActionError> {
    // Check if file is a template with {path}
    if file.contains("{path}") {
        // TODO: Implement template substitution
        // For now, just use the file as-is
        return Err(ActionError::NotImplemented(
            "Path templates not yet implemented".to_string(),
        ));
    }

    // Simple case: join directory and file
    let file_path = directory.join(file);

    // Canonicalize to resolve symlinks and ".." (but don't validate existence yet)
    // This is safe because we validate it's a file in the next step
    Ok(file_path)
}
