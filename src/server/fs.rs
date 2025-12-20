use crate::config::{AppConfig, RouteConfig};
use http::StatusCode;
use tokio::fs::{File, canonicalize, metadata};

pub async fn resolve_file(
    req_path: &str,
    config: &AppConfig,
    server_name: &str,
) -> Result<(StatusCode, Option<File>), StatusCode> {
    let server = config
        .servers
        .get(server_name)
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let normalized = if req_path == "/" {
        "/".to_string()
    } else {
        req_path.trim_end_matches('/').to_string()
    };

    if normalized != "/" {
        if let Some(segment) = normalized.rsplit('/').next() {
            if segment.contains('.') {
                return Err(StatusCode::NOT_FOUND);
            }
        }
    }

    let route: &RouteConfig = match server.routes.get(&normalized) {
        Some(r) => r,
        None => return Err(StatusCode::NOT_FOUND),
    };

    let canonical_dir = canonicalize(&route.directory)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let meta = metadata(&canonical_dir)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    if !meta.is_dir() {
        return Err(StatusCode::NOT_FOUND);
    }

    let file_path = canonical_dir.join(&route.file);

    let file = File::open(&file_path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok((StatusCode::OK, Some(file)))
}
