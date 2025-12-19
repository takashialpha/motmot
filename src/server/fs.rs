use http::StatusCode;
use std::path::PathBuf;
use tokio::fs::File;

pub async fn determine_file(
    req: &http::Request<()>,
    root: Option<&PathBuf>,
) -> Result<(StatusCode, Option<File>), StatusCode> {
    match root {
        None => Ok((StatusCode::OK, None)),
        Some(root) if req.uri().path().contains("..") => Err(StatusCode::NOT_FOUND),
        Some(root) => {
            let path = root.join(req.uri().path().trim_start_matches('/'));
            match File::open(&path).await {
                Ok(f) => Ok((StatusCode::OK, Some(f))),
                Err(_) => Err(StatusCode::NOT_FOUND),
            }
        }
    }
}
