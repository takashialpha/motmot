use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub servers: HashMap<String, Server>,
    pub logging: Logging,

    #[serde(default)]
    pub health: Health,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Server {
    /// IPv6-only host, e.g. "::"
    pub host: String,
    pub port: u16,

    /// Optional certificate paths - if None, will auto-generate self-signed
    #[serde(default)]
    pub cert_path: Option<PathBuf>,

    #[serde(default)]
    pub key_path: Option<PathBuf>,

    /// Enable WebTransport support (requires compatible browser/client)
    #[serde(default)]
    pub webtransport: bool,

    /// Dynamic route configuration
    #[serde(default)]
    pub routes: HashMap<String, RouteConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RouteConfig {
    /// Action to perform for GET requests
    #[serde(default)]
    pub get: Option<Action>,

    /// Action to perform for POST requests
    #[serde(default)]
    pub post: Option<Action>,

    /// Action to perform for PUT requests
    #[serde(default)]
    pub put: Option<Action>,

    /// Action to perform for DELETE requests
    #[serde(default)]
    pub delete: Option<Action>,

    /// Action to perform for PATCH requests
    #[serde(default)]
    pub patch: Option<Action>,

    /// Action to perform for HEAD requests (if None, uses GET action)
    #[serde(default)]
    pub head: Option<Action>,

    /// Action to perform for OPTIONS requests
    #[serde(default)]
    pub options: Option<Action>,

    /// Fallback action if method not explicitly configured
    #[serde(default)]
    pub fallback: Option<Action>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Action {
    /// Serve a static file from filesystem
    Static {
        /// Directory to serve from
        directory: PathBuf,

        /// File to serve (supports template: "{path}" for dynamic paths)
        file: String,

        /// Enable in-memory caching
        #[serde(default)]
        cache: bool,
    },

    /// Proxy request to upstream server
    Proxy {
        /// Upstream URL
        upstream: String,

        /// Preserve original Host header
        #[serde(default = "default_true")]
        preserve_host: bool,

        /// Request timeout in seconds
        #[serde(default = "default_proxy_timeout")]
        timeout_secs: u64,

        /// Strip path prefix before forwarding (e.g., "/api" -> "")
        #[serde(default)]
        strip_prefix: Option<String>,
    },

    /// Return a fixed JSON response
    Json {
        /// JSON body as string (will be parsed and validated)
        body: String,

        /// HTTP status code
        #[serde(default = "default_status_ok")]
        status: u16,
    },

    /// Return a fixed text response
    Text {
        /// Response body
        body: String,

        /// Content-Type header
        #[serde(default = "default_content_type_text")]
        content_type: String,

        /// HTTP status code
        #[serde(default = "default_status_ok")]
        status: u16,
    },

    /// Redirect to another URL
    Redirect {
        /// Target URL
        to: String,

        /// Status code (301, 302, 307, 308)
        #[serde(default = "default_redirect_status")]
        status: u16,
    },

    /// Execute a script and return its output (future feature)
    #[cfg(feature = "scripting")]
    Script {
        /// Path to script file
        script: PathBuf,

        /// Script interpreter (e.g., "lua", "rhai")
        interpreter: String,

        /// Timeout in seconds
        #[serde(default = "default_script_timeout")]
        timeout_secs: u64,
    },

    /// Explicitly deny the request
    Deny {
        /// Status code to return
        #[serde(default = "default_deny_status")]
        status: u16,

        /// Optional message
        message: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Health {
    /// Enable health checks on startup
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Timeout for all health checks in seconds
    #[serde(default = "default_health_timeout")]
    pub timeout_secs: u64,

    /// Check if certificates are valid (or can be generated)
    #[serde(default = "default_true")]
    pub check_certs: bool,

    /// Check if directories are accessible
    #[serde(default = "default_true")]
    pub check_directories: bool,

    /// Check if ports are available
    #[serde(default = "default_true")]
    pub check_ports: bool,
}

impl Default for Health {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout_secs: 30,
            check_certs: true,
            check_directories: true,
            check_ports: true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Logging {
    #[serde(default = "Logging::default_filter")]
    pub filter: String,

    #[serde(default)]
    pub file_path: Option<PathBuf>,
}

impl Logging {
    fn default_filter() -> String {
        "info,motmot=info,quinn=info,h3=info,h3_quinn=info".to_string()
    }
}

// Helper functions for serde defaults
fn default_true() -> bool {
    true
}

fn default_proxy_timeout() -> u64 {
    30
}

fn default_health_timeout() -> u64 {
    30
}

fn default_status_ok() -> u16 {
    200
}

fn default_content_type_text() -> String {
    "text/plain; charset=utf-8".to_string()
}

fn default_redirect_status() -> u16 {
    302
}

#[cfg(feature = "scripting")]
fn default_script_timeout() -> u64 {
    5
}

fn default_deny_status() -> u16 {
    403
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
                get: Some(Action::Static {
                    directory: data_dir.clone(),
                    file: "index.html".to_string(),
                    cache: false,
                }),
                head: None, // Will use GET action
                post: None,
                put: None,
                delete: None,
                patch: None,
                options: None,
                fallback: Some(Action::Deny {
                    status: 405,
                    message: Some("Method not allowed".to_string()),
                }),
            },
        );

        let mut servers = HashMap::new();
        servers.insert(
            "main".to_string(),
            Server {
                host: "::".to_string(),
                port: 443,
                cert_path: Some(ssl_dir.join("server.cert")),
                key_path: Some(ssl_dir.join("server.key")),
                webtransport: false,
                routes,
            },
        );

        Self {
            servers,
            logging: Logging {
                filter: Logging::default_filter(),
                file_path: Some(log_dir.join("motmot.log")),
            },
            health: Health::default(),
        }
    }
}
