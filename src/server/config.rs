use std::{net::SocketAddr, path::PathBuf, sync::Arc};

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub root: Option<Arc<PathBuf>>,
    pub listen: SocketAddr,
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}

impl ServerConfig {
    pub fn from_app_config(server: &crate::config::Server) -> std::io::Result<Self> {
        let listen = format!("{}:{}", server.host, server.port)
            .parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

        let root = server.root.as_ref().map(|p| Arc::new(p.clone()));

        Ok(Self {
            root,
            listen,
            cert_path: server.cert_path.clone(),
            key_path: server.key_path.clone(),
        })
    }
}
