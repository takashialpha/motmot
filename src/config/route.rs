use super::Action;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RouteConfig {
    // map http methods to an action
    #[serde(default)]
    pub methods: HashMap<String, Action>,
}
