use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Logging {
    #[serde(default = "Logging::default_filter")]
    pub filter: String,

    #[serde(default)]
    pub file: Option<PathBuf>,
}

impl Logging {
    pub fn default_filter() -> String {
        "info".to_string()
    }
}
