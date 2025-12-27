use super::Action;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RouteConfig {
    /// Map HTTP methods (GET, POST, etc.) to Actions
    #[serde(default)]
    pub methods: HashMap<String, Action>,

    /// Fallback action if method not explicitly configured
    #[serde(default)]
    pub fallback: Option<Action>,
}
