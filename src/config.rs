use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: Server,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Server {
    pub host: String,
    pub port: u16,

    /// Optional directory to serve files from
    #[serde(default)]
    pub root: Option<PathBuf>,

    /// Path to TLS certificate (DER or PEM, depending on your loader)
    pub cert_path: PathBuf,

    /// Path to TLS private key
    pub key_path: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: Server {
                host: "0.0.0.0".into(),
                port: 8080,

                root: Some(PathBuf::from("~/.config/motmot/index.html")),
                cert_path: PathBuf::from("~/.config/motmot/server.cert"),
                key_path: PathBuf::from("~/.config/motmot/server.key"),
            },
        }
    }
}
