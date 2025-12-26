use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PathError {
    #[error("path traversal attempt detected: {path}")]
    Traversal { path: String },

    #[error("invalid path: {path}")]
    Invalid { path: String },

    #[error("path does not exist: {path}")]
    NotFound { path: String },

    #[error("I/O error accessing path: {0}")]
    Io(#[from] std::io::Error),
}

/// Safely join a user-provided path to a base directory
///
/// Prevents path traversal attacks by:
/// - Canonicalizing both paths
/// - Ensuring result is within base directory
/// - Rejecting paths with ".." components
///
/// # Example
/// ```ignore
/// let base = Path::new("/var/www");
/// let user_path = "../../etc/passwd";
/// assert!(safe_join(base, user_path).is_err()); // Traversal detected
///
/// let safe_path = "index.html";
/// let result = safe_join(base, safe_path)?; // /var/www/index.html
/// ```
pub async fn safe_join(base: &Path, user_path: &str) -> Result<PathBuf, PathError> {
    // Reject obviously dangerous patterns
    if user_path.contains("..") {
        return Err(PathError::Traversal {
            path: user_path.to_string(),
        });
    }

    // Normalize user path (remove leading slash, multiple slashes)
    let normalized = normalize_user_path(user_path);

    // Join with base
    let joined = base.join(normalized);

    // Canonicalize to resolve symlinks and ".." components
    let canonical_joined = tokio::fs::canonicalize(&joined).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            PathError::NotFound {
                path: joined.display().to_string(),
            }
        } else {
            PathError::Io(e)
        }
    })?;

    // Ensure canonical base directory
    let canonical_base = tokio::fs::canonicalize(base).await?;

    // Verify result is within base directory
    if !canonical_joined.starts_with(&canonical_base) {
        return Err(PathError::Traversal {
            path: user_path.to_string(),
        });
    }

    Ok(canonical_joined)
}

/// Normalize a user-provided path for safe joining
///
/// - Removes leading slashes
/// - Collapses multiple slashes
/// - Trims whitespace
fn normalize_user_path(path: &str) -> String {
    path.trim()
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

/// Validate that a path exists and is a regular file
pub async fn validate_file(path: &Path) -> Result<(), PathError> {
    let metadata = tokio::fs::metadata(path).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            PathError::NotFound {
                path: path.display().to_string(),
            }
        } else {
            PathError::Io(e)
        }
    })?;

    if !metadata.is_file() {
        return Err(PathError::Invalid {
            path: path.display().to_string(),
        });
    }

    Ok(())
}

/// Validate that a path exists and is a directory
pub async fn validate_directory(path: &Path) -> Result<(), PathError> {
    let metadata = tokio::fs::metadata(path).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            PathError::NotFound {
                path: path.display().to_string(),
            }
        } else {
            PathError::Io(e)
        }
    })?;

    if !metadata.is_dir() {
        return Err(PathError::Invalid {
            path: path.display().to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_user_path() {
        assert_eq!(normalize_user_path("/foo/bar"), "foo/bar");
        assert_eq!(normalize_user_path("foo/bar"), "foo/bar");
        assert_eq!(normalize_user_path("//foo//bar//"), "foo/bar");
        assert_eq!(normalize_user_path("  foo/bar  "), "foo/bar");
        assert_eq!(normalize_user_path("/"), "");
    }

    #[test]
    fn test_detect_traversal() {
        assert!(normalize_user_path("../etc/passwd").contains(".."));
        assert!(normalize_user_path("foo/../../etc").contains(".."));
        assert!(!normalize_user_path("foo/bar").contains(".."));
    }
}
