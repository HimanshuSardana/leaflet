//! Typed, fail-fast server configuration sourced from the environment.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind: SocketAddr,
    pub data_dir: PathBuf,
    pub database_url: String,
    /// Where the exported web bundle lives. Absent in api-only deployments.
    pub web_dir: PathBuf,
    pub log: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{0} must be a socket address like 0.0.0.0:8080")]
    BadBind(String),
    #[error("{0} must be an ip address")]
    BadIp(String),
    #[error("LEAFLET_DATA_DIR is empty")]
    EmptyDataDir,
}

impl Config {
    /// Reads `LEAFLET_*` variables. `LEAFLET_DATABASE_URL` defaults to
    /// `$LEAFLET_DATA_DIR/leaflet.db`; everything else has a dev default.
    pub fn from_env() -> Result<Self, ConfigError> {
        let data_dir = std::env::var("LEAFLET_DATA_DIR").unwrap_or_else(|_| "./data".into());
        if data_dir.trim().is_empty() {
            return Err(ConfigError::EmptyDataDir);
        }
        let data_dir = PathBuf::from(data_dir);

        let bind = match std::env::var("LEAFLET_BIND") {
            Ok(raw) => raw.parse().map_err(|_| ConfigError::BadBind(raw))?,
            Err(_) => SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 8080),
        };

        let database_url = match std::env::var("LEAFLET_DATABASE_URL") {
            Ok(url) => url,
            Err(_) => format!(
                "sqlite://{}?mode=rwc",
                data_dir.join("leaflet.db").display()
            ),
        };

        let web_dir = std::env::var("LEAFLET_WEB_DIR").unwrap_or_else(|_| "./apps/web/out".into());
        let log = std::env::var("LEAFLET_LOG").unwrap_or_else(|_| "info,tower_http=debug".into());

        Ok(Self {
            bind,
            data_dir,
            database_url,
            web_dir: PathBuf::from(web_dir),
            log,
        })
    }

    pub fn blobs_dir(&self) -> PathBuf {
        self.data_dir.join("blobs")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // env vars are process-global; these assertions use the defaults only.
    #[test]
    fn blobs_dir_sits_under_data_dir() {
        let cfg = Config {
            bind: "0.0.0.0:8080".parse().unwrap(),
            data_dir: PathBuf::from("/var/lib/leaflet"),
            database_url: String::new(),
            web_dir: PathBuf::from("./apps/web/out"),
            log: String::new(),
        };
        assert_eq!(cfg.blobs_dir(), PathBuf::from("/var/lib/leaflet/blobs"));
    }
}
