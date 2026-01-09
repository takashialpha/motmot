pub mod action;
pub mod health;
pub mod logging;
pub mod route;
pub mod server;
pub mod standard;

pub use action::Action;
pub use health::Health;
pub use logging::Logging;
pub use route::RouteConfig;
pub use server::Server;
pub use standard::StandardResponses;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub servers: HashMap<String, Server>,
    pub logging: Logging,

    #[serde(default)]
    pub health: Health,
}

impl Default for AppConfig {
    fn default() -> Self {
        use std::path::PathBuf;

        let data_dir = PathBuf::from("/var/lib/motmot");
        let log_dir = PathBuf::from("/var/log/motmot");

        let mut methods = std::collections::HashMap::new();
        methods.insert(
            "GET".to_string(),
            Action::Static {
                path: data_dir.join("index.html"),
                cache: false, // change to true when cache get well-implemented.
            },
        );

        let mut routes = std::collections::HashMap::new();
        routes.insert("/".to_string(), RouteConfig { methods });

        let mut servers = std::collections::HashMap::new();
        servers.insert(
            "main".to_string(),
            Server {
                host: "::".to_string(),
                port: 443,
                tls: None,
                webtransport: false,
                routes,
                standard: standard::StandardResponses::default(),
            },
        );

        Self {
            servers,
            logging: Logging {
                filter: logging::Logging::default_filter(),
                file: Some(log_dir.join("motmot.log")),
            },
            health: health::Health::default(),
        }
    }
}
