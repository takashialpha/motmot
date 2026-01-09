use crate::config::Action;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StandardResponses {
    pub not_found: Action,
    pub method_not_allowed: Action,
    pub internal_error: Action,
}

impl Default for StandardResponses {
    fn default() -> Self {
        Self {
            not_found: Action::Response {
                body: "Not Found".into(),
                content_type: "text/plain; charset=utf-8".into(),
                status: 404,
            },
            method_not_allowed: Action::Response {
                body: "Method Not Allowed".into(),
                content_type: "text/plain; charset=utf-8".into(),
                status: 405,
            },
            internal_error: Action::Response {
                body: "Internal Server Error".into(),
                content_type: "text/plain; charset=utf-8".into(),
                status: 500,
            },
        }
    }
}
