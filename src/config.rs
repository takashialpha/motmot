use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub server: Server,
    pub logging: Logging,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Server {
    pub host: String,
    pub port: u16,

    #[serde(default)]
    pub root: Option<PathBuf>,

    pub cert_path: PathBuf,
    pub key_path: PathBuf,

    /// Dynamic route configuration
    #[serde(default)]
    pub routes: HashMap<String, RouteConfig>,
}

/// Route configuration for a single path
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RouteConfig {
    /// relative to server root
    pub directory: PathBuf,
    /// file to serve in this directory
    pub file: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Logging {
    #[serde(default = "Logging::default_stdout_level")]
    pub stdout_level: String,

    #[serde(default = "Logging::default_file_level")]
    pub file_level: String,

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
        let config_dir = PathBuf::from("/etc/motmot");
        let ssl_dir = config_dir.join("ssl");

        let data_dir = PathBuf::from("/var/lib/motmot");
        let log_dir = PathBuf::from("/var/log/motmot");

        // Default routes map
        let mut routes = std::collections::HashMap::new();

        // "/" -> root directory, default file index.html
        routes.insert(
            "/".to_string(),
            RouteConfig {
                directory: data_dir.clone(), // root dir
                file: "index.html".to_string(),
            },
        );

        Self {
            server: Server {
                host: "0.0.0.0".into(),
                port: 443,
                root: Some(data_dir.clone()),
                cert_path: ssl_dir.join("server.cert"),
                key_path: ssl_dir.join("server.key"),
                routes,
            },
            logging: Logging {
                stdout_level: Logging::default_stdout_level(),
                file_level: Logging::default_file_level(),
                file_path: Some(log_dir.join("motmot.log")),
            },
        }
    }
}
