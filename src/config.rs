use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: Server,
    pub logging: Logging,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Server {
    pub host: String,
    pub port: u16,

    #[serde(default)]
    pub root: Option<PathBuf>,

    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Logging {
    /// Level for stdout/stderr output
    #[serde(default = "Logging::default_stdout_level")]
    pub stdout_level: String,

    /// Level for file output
    #[serde(default = "Logging::default_file_level")]
    pub file_level: String,

    /// Optional path to log file
    #[serde(default)]
    pub file_path: Option<PathBuf>,
}

impl Logging {
    fn default_stdout_level() -> String {
        "info".to_string()
    }

    fn default_file_level() -> String {
        "debug".to_string()
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("motmot");

        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("motmot");

        Self {
            server: Server {
                host: "0.0.0.0".into(),
                port: 8080,
                root: Some(data_dir.clone()),
                cert_path: config_dir.join("server.cert"),
                key_path: config_dir.join("server.key"),
            },
            logging: Logging {
                stdout_level: Logging::default_stdout_level(),
                file_level: Logging::default_file_level(),
                file_path: Some(data_dir.join("app.log")),
            },
        }
    }
}
