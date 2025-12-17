use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: Server,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Server {
    pub host: String,
    pub port: u16,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: Server {
                host: "0.0.0.0".into(),
                port: 8080,
            },
        }
    }
}
