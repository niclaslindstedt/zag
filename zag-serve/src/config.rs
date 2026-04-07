//! Server configuration for zag serve.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Server configuration loaded from ~/.zag/serve.toml.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServeConfig {
    #[serde(default)]
    pub server: ServerSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSection {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub token: Option<String>,
    pub tls_cert: Option<String>,
    pub tls_key: Option<String>,
    /// When true, all connected users' agent sessions are forced to run inside a Docker sandbox.
    #[serde(default)]
    pub force_sandbox: bool,
}

impl Default for ServerSection {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            token: None,
            tls_cert: None,
            tls_key: None,
            force_sandbox: false,
        }
    }
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    2100
}

impl ServeConfig {
    /// Path to the serve config file.
    pub fn config_path() -> PathBuf {
        zag_agent::config::Config::global_base_dir().join("serve.toml")
    }

    /// Load config from ~/.zag/serve.toml. Returns default if not found.
    pub fn load() -> Self {
        let path = Self::config_path();
        match std::fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save config to ~/.zag/serve.toml.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}

/// Connection configuration stored in ~/.zag/connect.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectConfig {
    pub url: String,
    pub token: String,
    /// Username of the authenticated user (present in user-account mode).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}

impl ConnectConfig {
    /// Path to the connect config file.
    pub fn config_path() -> PathBuf {
        zag_agent::config::Config::global_base_dir().join("connect.json")
    }

    /// Load active connection config. Returns None if not connected.
    pub fn load() -> Option<Self> {
        let path = Self::config_path();
        let content = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Save connection config (marks as connected).
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Remove connection config (disconnect).
    pub fn remove() -> Result<()> {
        let path = Self::config_path();
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        // Also clean up the health cache
        let cache_path = Self::health_cache_path();
        if cache_path.exists() {
            let _ = std::fs::remove_file(&cache_path);
        }
        Ok(())
    }

    /// Path to the health check cache file.
    pub fn health_cache_path() -> PathBuf {
        zag_agent::config::Config::global_base_dir().join("health_cache")
    }

    /// Check if the health cache is still valid (within `ttl_secs` of now).
    pub fn is_health_cache_valid(ttl_secs: u64) -> bool {
        let path = Self::health_cache_path();
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return false,
        };
        let cached_ts: u64 = match content.trim().parse() {
            Ok(ts) => ts,
            Err(_) => return false,
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now.saturating_sub(cached_ts) < ttl_secs
    }

    /// Update the health cache with the current timestamp.
    pub fn update_health_cache() -> Result<()> {
        let path = Self::health_cache_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        std::fs::write(&path, now.to_string())?;
        Ok(())
    }

    /// Check if currently connected to a remote server.
    pub fn is_connected() -> bool {
        Self::config_path().exists()
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
