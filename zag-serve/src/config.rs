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
}

impl Default for ServerSection {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            token: None,
            tls_cert: None,
            tls_key: None,
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
