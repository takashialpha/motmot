use super::RouteConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Server {
    pub host: String,
    pub port: u16,

    #[serde(default)]
    pub tls: Option<ServerTlsConf>,

    #[serde(default)]
    pub webtransport: bool,

    #[serde(default)]
    pub routes: HashMap<String, RouteConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerTlsConf {
    pub cert: std::path::PathBuf,
    pub key: std::path::PathBuf,
}
