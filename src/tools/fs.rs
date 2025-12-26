use std::path::Path;

use bytes::{Bytes, BytesMut};
use tokio::fs::File;
use tokio::io::AsyncReadExt;

/// Chunk size for streaming files (40KB)
const CHUNK_SIZE: usize = 1024 * 40;

/// Read a file in chunks for streaming
///
/// Returns an async iterator-like structure that yields chunks of bytes.
/// Used by action executors when serving static files.
pub struct FileStream {
    file: File,
    buffer: BytesMut,
}

impl FileStream {
    /// Open a file for streaming
    pub async fn open(path: &Path) -> std::io::Result<Self> {
        let file = File::open(path).await?;
        let buffer = BytesMut::with_capacity(CHUNK_SIZE);

        Ok(Self { file, buffer })
    }

    /// Read the next chunk from the file
    ///
    /// Returns None when EOF is reached
    pub async fn next_chunk(&mut self) -> std::io::Result<Option<Bytes>> {
        self.buffer.clear();
        self.buffer.reserve(CHUNK_SIZE);

        // Read up to CHUNK_SIZE bytes
        let n = self.file.read_buf(&mut self.buffer).await?;

        if n == 0 {
            Ok(None)
        } else {
            Ok(Some(self.buffer.split().freeze()))
        }
    }

    /// Get file metadata
    pub async fn metadata(&self) -> std::io::Result<std::fs::Metadata> {
        self.file.metadata().await
    }
}

/// Read entire file into memory (use sparingly, mainly for small responses)
pub async fn read_to_bytes(path: &Path) -> std::io::Result<Bytes> {
    let contents = tokio::fs::read(path).await?;
    Ok(Bytes::from(contents))
}

/// Get file size without reading contents
pub async fn file_size(path: &Path) -> std::io::Result<u64> {
    let metadata = tokio::fs::metadata(path).await?;
    Ok(metadata.len())
}

/// Guess MIME type from file extension
pub fn guess_mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        // Text
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("xml") => "application/xml",
        Some("txt") => "text/plain; charset=utf-8",
        Some("csv") => "text/csv",

        // Images
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",

        // Fonts
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("otf") => "font/otf",

        // Video
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",

        // Audio
        Some("mp3") => "audio/mpeg",
        Some("ogg") => "audio/ogg",
        Some("wav") => "audio/wav",

        // Archives
        Some("zip") => "application/zip",
        Some("gz") => "application/gzip",
        Some("tar") => "application/x-tar",

        // Documents
        Some("pdf") => "application/pdf",

        // Default
        _ => "application/octet-stream",
    }
}
