//! Configuration management for the agent CLI.
//!
//! Configuration is stored in `.agent/agent.toml` in the project root
//! (or `--root` directory if specified).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Agent-specific model configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentModels {
    pub claude: Option<String>,
    pub codex: Option<String>,
    pub gemini: Option<String>,
    pub copilot: Option<String>,
}

/// Default settings applied when not overridden by CLI flags.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Defaults {
    /// Default agent to use for workflows (claude, codex, gemini, copilot)
    pub agent: Option<String>,
    /// Auto-approve all actions (skip permission prompts)
    pub auto_approve: Option<bool>,
}

/// Root configuration structure.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// Default settings
    #[serde(default)]
    pub defaults: Defaults,
    /// Per-agent model defaults
    #[serde(default)]
    pub models: AgentModels,
}

impl Config {
    /// Load configuration from the `.agent/agent.toml` file.
    ///
    /// If `root` is provided, looks in `<root>/.agent/agent.toml`.
    /// Otherwise uses current working directory.
    ///
    /// Returns default config if file doesn't exist.
    pub fn load(root: Option<&str>) -> Result<Self> {
        let path = Self::config_path(root);
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config: {}", path.display()))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config: {}", path.display()))?;
        Ok(config)
    }

    /// Save configuration to the `.agent/agent.toml` file.
    ///
    /// Creates the `.agent` directory if it doesn't exist.
    pub fn save(&self, root: Option<&str>) -> Result<()> {
        let path = Self::config_path(root);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write config: {}", path.display()))?;
        Ok(())
    }

    /// Initialize config file with defaults if it doesn't exist.
    ///
    /// Returns true if a new config was created, false if it already existed.
    pub fn init(root: Option<&str>) -> Result<bool> {
        let path = Self::config_path(root);
        if path.exists() {
            return Ok(false);
        }

        let config = Self::default_with_comments();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        std::fs::write(&path, config)
            .with_context(|| format!("Failed to write config: {}", path.display()))?;
        Ok(true)
    }

    /// Get the path to the config file.
    pub fn config_path(root: Option<&str>) -> PathBuf {
        let base = root
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        base.join(".agent").join("agent.toml")
    }

    /// Get the .agent directory path.
    pub fn agent_dir(root: Option<&str>) -> PathBuf {
        let base = root
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        base.join(".agent")
    }

    /// Ensure the .agent directory exists.
    pub fn ensure_agent_dir(root: Option<&str>) -> Result<PathBuf> {
        let dir = Self::agent_dir(root);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create .agent directory: {}", dir.display()))?;
        Ok(dir)
    }

    /// Get the default model for a specific agent, if configured.
    pub fn get_model(&self, agent: &str) -> Option<&str> {
        match agent {
            "claude" => self.models.claude.as_deref(),
            "codex" => self.models.codex.as_deref(),
            "gemini" => self.models.gemini.as_deref(),
            "copilot" => self.models.copilot.as_deref(),
            _ => None,
        }
    }

    /// Check if auto-approve is enabled by default.
    pub fn auto_approve(&self) -> bool {
        self.defaults.auto_approve.unwrap_or(false)
    }

    /// Get the default agent, if configured.
    pub fn default_agent(&self) -> Option<&str> {
        self.defaults.agent.as_deref()
    }

    /// Generate default config content with comments.
    fn default_with_comments() -> String {
        r#"# Agent CLI Configuration
# This file configures default behavior for the agent CLI.
# Settings here can be overridden by command-line flags.

[defaults]
# Default agent to use for workflows (claude, codex, gemini, copilot)
# agent = "claude"

# Auto-approve all actions (skip permission prompts)
# auto_approve = false

[models]
# Default models for each agent (overrides agent defaults)
# claude = "opus"
# codex = "gpt-5.2-codex"
# gemini = "auto"
# copilot = "claude-sonnet-4.5"
"#
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.defaults.agent.is_none());
        assert!(config.defaults.auto_approve.is_none());
        assert!(config.models.claude.is_none());
    }

    #[test]
    fn test_parse_config() {
        let toml = r#"
[defaults]
agent = "claude"
auto_approve = true

[models]
claude = "sonnet"
codex = "gpt-5.1-codex-mini"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.defaults.agent, Some("claude".to_string()));
        assert_eq!(config.defaults.auto_approve, Some(true));
        assert_eq!(config.models.claude, Some("sonnet".to_string()));
        assert_eq!(config.models.codex, Some("gpt-5.1-codex-mini".to_string()));
        assert!(config.models.gemini.is_none());
    }

    #[test]
    fn test_get_model() {
        let config = Config {
            models: AgentModels {
                claude: Some("opus".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(config.get_model("claude"), Some("opus"));
        assert_eq!(config.get_model("codex"), None);
    }
}
