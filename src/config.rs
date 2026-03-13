//! Configuration management for the agent CLI.
//!
//! Configuration is stored in `.agent/agent.toml` in the project root
//! (or `--root` directory if specified).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

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
    /// Auto-approve all actions (skip permission prompts)
    pub auto_approve: Option<bool>,
    /// Default model size for all agents (small, medium, large)
    pub model: Option<String>,
    /// Default provider (claude, codex, gemini, copilot)
    pub provider: Option<String>,
}

/// Auto-selection configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AutoConfig {
    /// Provider used for auto-selection (default: "claude")
    pub provider: Option<String>,
    /// Model used for auto-selection (default: "haiku")
    pub model: Option<String>,
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
    /// Auto-selection settings
    #[serde(default)]
    pub auto: AutoConfig,
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
    /// Also ensures `.agent/` is added to `.gitignore` if it isn't already.
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

        // Ensure .agent/ is in .gitignore
        Self::ensure_gitignore(root)?;

        Ok(true)
    }

    /// Ensure `.agent/` is added to `.gitignore` if it isn't already.
    /// Only applies when the config is stored in a git repository.
    fn ensure_gitignore(root: Option<&str>) -> Result<()> {
        let base = Self::resolve_base_dir(root);

        // Only add to .gitignore if we're in a git repository
        // (i.e., not using global config directory)
        if let Some(r) = root {
            // Explicit root was provided - check if it's a git repo
            if Self::find_git_root(&PathBuf::from(r)).is_none() {
                return Ok(());
            }
        } else {
            let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            if Self::find_git_root(&current_dir).is_none() {
                // Not in a git repo, using global config - no .gitignore needed
                return Ok(());
            }
        }

        let gitignore_path = base.join(".gitignore");

        let content = if gitignore_path.exists() {
            std::fs::read_to_string(&gitignore_path).with_context(|| {
                format!("Failed to read .gitignore: {}", gitignore_path.display())
            })?
        } else {
            String::new()
        };

        // Check if .agent/ is already in .gitignore
        let has_agent_entry = content.lines().any(|line| {
            let trimmed = line.trim();
            trimmed == ".agent"
                || trimmed == ".agent/"
                || trimmed == "/.agent"
                || trimmed == "/.agent/"
        });

        if !has_agent_entry {
            let new_content = if content.is_empty() {
                "# Agent CLI state directory\n.agent/\n".to_string()
            } else if content.ends_with('\n') {
                format!("{}\n# Agent CLI state directory\n.agent/\n", content)
            } else {
                format!("{}\n\n# Agent CLI state directory\n.agent/\n", content)
            };

            std::fs::write(&gitignore_path, new_content).with_context(|| {
                format!("Failed to write .gitignore: {}", gitignore_path.display())
            })?;
        }

        Ok(())
    }

    /// Detect git repository root from a given directory.
    /// Returns None if not in a git repository.
    fn find_git_root(start_dir: &Path) -> Option<PathBuf> {
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("--show-toplevel")
            .current_dir(start_dir)
            .output()
            .ok()?;

        if output.status.success() {
            let root = String::from_utf8(output.stdout).ok()?;
            Some(PathBuf::from(root.trim()))
        } else {
            None
        }
    }

    /// Get the global config directory (~/.config/agent on Unix, ~/AppData/Roaming/agent on Windows).
    fn global_config_dir() -> PathBuf {
        if cfg!(target_os = "windows") {
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("agent")
        } else {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("agent")
        }
    }

    /// Resolve the base directory for config storage.
    /// Priority:
    /// 1. Explicit root parameter if provided
    /// 2. Git repository root if current directory is in a repo
    /// 3. Global config directory (~/.config/agent)
    fn resolve_base_dir(root: Option<&str>) -> PathBuf {
        if let Some(r) = root {
            return PathBuf::from(r);
        }

        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // Try to find git root
        if let Some(git_root) = Self::find_git_root(&current_dir) {
            return git_root;
        }

        // Fall back to global config directory
        Self::global_config_dir()
    }

    /// Get the path to the config file.
    pub fn config_path(root: Option<&str>) -> PathBuf {
        let base = Self::resolve_base_dir(root);
        base.join(".agent").join("agent.toml")
    }

    /// Get the .agent directory path.
    #[allow(dead_code)]
    pub fn agent_dir(root: Option<&str>) -> PathBuf {
        let base = Self::resolve_base_dir(root);
        base.join(".agent")
    }

    /// Get the global logs directory path.
    /// Always uses the global config dir so logs are centralized.
    pub fn global_logs_dir() -> PathBuf {
        Self::global_config_dir().join(".agent").join("logs")
    }

    /// Ensure the .agent directory exists.
    #[allow(dead_code)]
    pub fn ensure_agent_dir(root: Option<&str>) -> Result<PathBuf> {
        let dir = Self::agent_dir(root);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create .agent directory: {}", dir.display()))?;
        Ok(dir)
    }

    /// Get the default model for a specific agent, if configured.
    /// Checks agent-specific model first, then falls back to defaults.model.
    pub fn get_model(&self, agent: &str) -> Option<&str> {
        // First check agent-specific model
        let agent_model = match agent {
            "claude" => self.models.claude.as_deref(),
            "codex" => self.models.codex.as_deref(),
            "gemini" => self.models.gemini.as_deref(),
            "copilot" => self.models.copilot.as_deref(),
            _ => None,
        };

        // Return agent-specific model if set, otherwise fall back to default
        agent_model.or(self.defaults.model.as_deref())
    }

    /// Get the global default model (without agent-specific override).
    #[allow(dead_code)]
    pub fn default_model(&self) -> Option<&str> {
        self.defaults.model.as_deref()
    }

    /// Check if auto-approve is enabled by default.
    pub fn auto_approve(&self) -> bool {
        self.defaults.auto_approve.unwrap_or(false)
    }

    /// Get the default provider, if configured.
    pub fn provider(&self) -> Option<&str> {
        self.defaults.provider.as_deref()
    }

    /// Get the auto-selection provider, if configured.
    pub fn auto_provider(&self) -> Option<&str> {
        self.auto.provider.as_deref()
    }

    /// Get the auto-selection model, if configured.
    pub fn auto_model(&self) -> Option<&str> {
        self.auto.model.as_deref()
    }

    /// Valid provider names (including "auto").
    pub const VALID_PROVIDERS: &'static [&'static str] =
        &["claude", "codex", "gemini", "copilot", "auto"];

    /// Get a config value by dot-notation key.
    #[allow(dead_code)]
    pub fn get_value(&self, key: &str) -> Option<String> {
        match key {
            "provider" => self.defaults.provider.clone(),
            "model" => self.defaults.model.clone(),
            "auto_approve" => self.defaults.auto_approve.map(|v| v.to_string()),
            "model.claude" => self.models.claude.clone(),
            "model.codex" => self.models.codex.clone(),
            "model.gemini" => self.models.gemini.clone(),
            "model.copilot" => self.models.copilot.clone(),
            "auto.provider" => self.auto.provider.clone(),
            "auto.model" => self.auto.model.clone(),
            _ => None,
        }
    }

    /// Set a config value by dot-notation key. Validates inputs.
    pub fn set_value(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "provider" => {
                let v = value.to_lowercase();
                if !Self::VALID_PROVIDERS.contains(&v.as_str()) {
                    anyhow::bail!(
                        "Invalid provider '{}'. Available: {}",
                        value,
                        Self::VALID_PROVIDERS.join(", ")
                    );
                }
                self.defaults.provider = Some(v);
            }
            "model" => {
                self.defaults.model = Some(value.to_string());
            }
            "auto_approve" => match value.to_lowercase().as_str() {
                "true" | "1" | "yes" => self.defaults.auto_approve = Some(true),
                "false" | "0" | "no" => self.defaults.auto_approve = Some(false),
                _ => anyhow::bail!(
                    "Invalid value '{}' for auto_approve. Use true or false.",
                    value
                ),
            },
            "model.claude" => self.models.claude = Some(value.to_string()),
            "model.codex" => self.models.codex = Some(value.to_string()),
            "model.gemini" => self.models.gemini = Some(value.to_string()),
            "model.copilot" => self.models.copilot = Some(value.to_string()),
            "auto.provider" => self.auto.provider = Some(value.to_string()),
            "auto.model" => self.auto.model = Some(value.to_string()),
            _ => anyhow::bail!(
                "Unknown config key '{}'. Available: provider, model, auto_approve, model.claude, model.codex, model.gemini, model.copilot, auto.provider, auto.model",
                key
            ),
        }
        Ok(())
    }

    /// Generate default config content with comments.
    fn default_with_comments() -> String {
        r#"# Agent CLI Configuration
# This file configures default behavior for the agent CLI.
# Settings here can be overridden by command-line flags.

[defaults]
# Default provider (claude, codex, gemini, copilot)
# provider = "claude"

# Auto-approve all actions (skip permission prompts)
# auto_approve = false

# Default model size for all agents (small, medium, large)
# Can be overridden per-agent in [models] section
model = "medium"

[models]
# Default models for each agent (overrides defaults.model)
# Use size aliases (small, medium, large) or specific model names
# claude = "opus"
# codex = "gpt-5.2-codex"
# gemini = "auto"
# copilot = "claude-sonnet-4.5"

[auto]
# Settings for auto provider/model selection (-p auto / -m auto)
# provider = "claude"
# model = "haiku"
"#
        .to_string()
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
