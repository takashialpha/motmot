use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub servers: HashMap<String, Server>,
    pub logging: Logging,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Server {
    /// IPv6-only host, e.g. "::"
    pub host: String,
    pub port: u16,

    pub cert_path: PathBuf,
    pub key_path: PathBuf,

    /// Dynamic route configuration
    #[serde(default)]
    pub routes: HashMap<String, RouteConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RouteConfig {
    /// Absolute directory to serve from
    pub directory: PathBuf,

    /// File served for this route
    pub file: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Logging {
    #[serde(default = "Logging::default_level")]
    pub level: String,

    #[serde(default)]
    pub file_path: Option<PathBuf>,
}

impl Logging {
    fn default_level() -> String {
        "info".to_string()
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        let config_dir = PathBuf::from("/etc/motmot");
        let ssl_dir = config_dir.join("ssl");

        let data_dir = PathBuf::from("/var/lib/motmot");
        let log_dir = PathBuf::from("/var/log/motmot");

        let mut routes = HashMap::new();
        routes.insert(
            "/".to_string(),
            RouteConfig {
                directory: data_dir.clone(),
                file: "index.html".to_string(),
            },
        );

        let mut servers = HashMap::new();
        servers.insert(
            "main".to_string(),
            Server {
                host: "::".to_string(),
                port: 443,
                cert_path: ssl_dir.join("server.cert"),
                key_path: ssl_dir.join("server.key"),
                routes,
            },
        );

        Self {
            servers,
            logging: Logging {
                level: Logging::default_level(),
                file_path: Some(log_dir.join("motmot.log")),
            },
        }
    }
}
