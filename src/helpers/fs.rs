use std::path::Path;
use tokio::fs;

#[derive(Debug)]
pub enum StaticRead {
    Ok(Vec<u8>),
    NotFound,
    Forbidden,
    Error,
}

pub async fn read(path: &Path) -> StaticRead {
    match fs::read(path).await {
        Ok(data) => StaticRead::Ok(data),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => StaticRead::NotFound,
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => StaticRead::Forbidden,
        Err(_) => StaticRead::Error,
    }
}
