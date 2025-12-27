use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Action {
    Static {
        path: PathBuf,
        #[serde(default)]
        cache: bool,
    },

    Proxy {
        upstream: String,
    },

    Response {
        body: String,
        content_type: String,
        #[serde(default = "default_status_ok")]
        status: u16,
    },

    Script {
        script: PathBuf,
        interpreter: String,
    },
}

fn default_status_ok() -> u16 {
    200
}
