use crate::config::{AppConfig, RouteConfig};
use http::StatusCode;
use tokio::fs::{File, canonicalize, metadata};

pub async fn resolve_file(
    req_path: &str,
    config: &AppConfig,
) -> Result<(StatusCode, Option<File>), StatusCode> {
    let normalized = if req_path == "/" {
        "/".to_string()
    } else {
        req_path.trim_end_matches('/').to_string()
    };

    let mut matched_route: Option<&RouteConfig> = None;
    let mut max_len = 0;

    for (url, route) in &config.server.routes {
        if normalized == *url || (normalized.starts_with(url) && url != "/") {
            if url.len() > max_len {
                matched_route = Some(route);
                max_len = url.len();
            }
        }
    }

    let route = match matched_route {
        Some(r) => r,
        None => return Err(StatusCode::NOT_FOUND),
    };

    let dir_path = &route.directory;

    let canonical_dir = canonicalize(dir_path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let meta = metadata(&canonical_dir)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    if !meta.is_dir() {
        return Err(StatusCode::NOT_FOUND);
    }

    let default_file = canonical_dir.join(&route.file);
    let file = File::open(&default_file)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok((StatusCode::OK, Some(file)))
}
