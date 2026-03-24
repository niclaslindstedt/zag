//! Configuration management for the agent CLI.
//!
//! Configuration is stored in `~/.agent/projects/<sanitized-path>/agent.toml`,
//! where the sanitized path is derived from the git repository root or explicit `--root`.

use anyhow::{Context, Result};
use log::debug;
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
    pub ollama: Option<String>,
}

/// Ollama-specific configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OllamaConfig {
    /// Default model name (default: "qwen3.5")
    pub model: Option<String>,
    /// Default parameter size (default: "9b")
    pub size: Option<String>,
    /// Parameter size for small alias
    pub size_small: Option<String>,
    /// Parameter size for medium alias
    pub size_medium: Option<String>,
    /// Parameter size for large alias
    pub size_large: Option<String>,
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
    /// Model used for auto-selection (default: "sonnet")
    pub model: Option<String>,
}

/// Listen command configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListenConfig {
    /// Default output format: "text", "json", or "rich-text"
    pub format: Option<String>,
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
    /// Ollama-specific settings
    #[serde(default)]
    pub ollama: OllamaConfig,
    /// Listen command settings
    #[serde(default)]
    pub listen: ListenConfig,
}

impl Config {
    /// Load configuration from `~/.agent/projects/<id>/agent.toml`.
    ///
    /// The project ID is derived from the git repo root or explicit `--root`.
    /// Returns default config if file doesn't exist.
    pub fn load(root: Option<&str>) -> Result<Self> {
        let path = Self::config_path(root);
        debug!("Loading config from {}", path.display());
        if !path.exists() {
            debug!("Config file not found, using defaults");
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config: {}", path.display()))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config: {}", path.display()))?;
        debug!("Config loaded successfully from {}", path.display());
        Ok(config)
    }

    /// Save configuration to `~/.agent/projects/<id>/agent.toml`.
    ///
    /// Creates the directory if it doesn't exist.
    pub fn save(&self, root: Option<&str>) -> Result<()> {
        let path = Self::config_path(root);
        debug!("Saving config to {}", path.display());
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write config: {}", path.display()))?;
        debug!("Config saved to {}", path.display());
        Ok(())
    }

    /// Initialize config file with defaults if it doesn't exist.
    ///
    /// Returns true if a new config was created, false if it already existed.
    pub fn init(root: Option<&str>) -> Result<bool> {
        let path = Self::config_path(root);
        if path.exists() {
            debug!("Config already exists at {}", path.display());
            return Ok(false);
        }

        debug!("Initializing new config at {}", path.display());
        let config = Self::default_with_comments();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        std::fs::write(&path, config)
            .with_context(|| format!("Failed to write config: {}", path.display()))?;

        Ok(true)
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

    /// Get the global agent base directory (~/.agent).
    pub fn global_base_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".agent")
    }

    /// Sanitize an absolute path into a directory name.
    /// Strips leading `/` and replaces `/` with `-`.
    pub fn sanitize_path(path: &str) -> String {
        path.trim_start_matches('/').replace('/', "-")
    }

    /// Resolve the project directory for config/session storage.
    ///
    /// All state is stored under `~/.agent/`:
    /// - Per-project: `~/.agent/projects/<sanitized-path>/`
    /// - Global (no repo): `~/.agent/`
    fn resolve_project_dir(root: Option<&str>) -> PathBuf {
        let base = Self::global_base_dir();

        // Keep this helper free of logging. It is used by config/session path
        // resolution on hot paths, and debug logging here can re-enter the same
        // resolution flow through logger setup and formatting.
        if let Some(r) = root {
            let sanitized = Self::sanitize_path(r);
            return base.join("projects").join(sanitized);
        }

        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // Try to find git root
        if let Some(git_root) = Self::find_git_root(&current_dir) {
            let sanitized = Self::sanitize_path(&git_root.to_string_lossy());
            return base.join("projects").join(sanitized);
        }

        // Fall back to global base directory (no project subdir)
        base
    }

    /// Get the path to the config file.
    pub fn config_path(root: Option<&str>) -> PathBuf {
        Self::resolve_project_dir(root).join("agent.toml")
    }

    /// Get the project directory path (for sessions, etc.).
    #[allow(dead_code)]
    pub fn agent_dir(root: Option<&str>) -> PathBuf {
        Self::resolve_project_dir(root)
    }

    /// Get the global logs directory path.
    pub fn global_logs_dir() -> PathBuf {
        Self::global_base_dir().join("logs")
    }

    /// Ensure the project directory exists.
    #[allow(dead_code)]
    pub fn ensure_agent_dir(root: Option<&str>) -> Result<PathBuf> {
        let dir = Self::agent_dir(root);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create project directory: {}", dir.display()))?;
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
            "ollama" => self.models.ollama.as_deref(),
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

    /// Get the ollama model name (default: "qwen3.5").
    pub fn ollama_model(&self) -> &str {
        self.ollama.model.as_deref().unwrap_or("qwen3.5")
    }

    /// Get the ollama default size (default: "9b").
    pub fn ollama_size(&self) -> &str {
        self.ollama.size.as_deref().unwrap_or("9b")
    }

    /// Get the ollama size for a model size alias, with config override.
    pub fn ollama_size_for<'a>(&'a self, size: &'a str) -> &'a str {
        match size {
            "small" | "s" => self.ollama.size_small.as_deref().unwrap_or("2b"),
            "medium" | "m" | "default" => self.ollama.size_medium.as_deref().unwrap_or("9b"),
            "large" | "l" | "max" => self.ollama.size_large.as_deref().unwrap_or("35b"),
            _ => size, // passthrough for explicit sizes like "27b"
        }
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

    /// Get the listen output format, if configured.
    pub fn listen_format(&self) -> Option<&str> {
        self.listen.format.as_deref()
    }

    /// Valid provider names (including "auto").
    pub const VALID_PROVIDERS: &'static [&'static str] =
        &["claude", "codex", "gemini", "copilot", "ollama", "auto"];

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
            "model.ollama" => self.models.ollama.clone(),
            "auto.provider" => self.auto.provider.clone(),
            "auto.model" => self.auto.model.clone(),
            "ollama.model" => self.ollama.model.clone(),
            "ollama.size" => self.ollama.size.clone(),
            "ollama.size_small" => self.ollama.size_small.clone(),
            "ollama.size_medium" => self.ollama.size_medium.clone(),
            "ollama.size_large" => self.ollama.size_large.clone(),
            "listen.format" => self.listen.format.clone(),
            _ => None,
        }
    }

    /// Set a config value by dot-notation key. Validates inputs.
    pub fn set_value(&mut self, key: &str, value: &str) -> Result<()> {
        debug!("Setting config: {} = {}", key, value);
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
            "model.ollama" => self.models.ollama = Some(value.to_string()),
            "auto.provider" => self.auto.provider = Some(value.to_string()),
            "auto.model" => self.auto.model = Some(value.to_string()),
            "ollama.model" => self.ollama.model = Some(value.to_string()),
            "ollama.size" => self.ollama.size = Some(value.to_string()),
            "ollama.size_small" => self.ollama.size_small = Some(value.to_string()),
            "ollama.size_medium" => self.ollama.size_medium = Some(value.to_string()),
            "ollama.size_large" => self.ollama.size_large = Some(value.to_string()),
            "listen.format" => {
                let v = value.to_lowercase();
                if !["text", "json", "rich-text"].contains(&v.as_str()) {
                    anyhow::bail!(
                        "Invalid listen format '{}'. Available: text, json, rich-text",
                        value
                    );
                }
                self.listen.format = Some(v);
            }
            _ => anyhow::bail!(
                "Unknown config key '{}'. Available: provider, model, auto_approve, model.claude, model.codex, model.gemini, model.copilot, model.ollama, auto.provider, auto.model, ollama.model, ollama.size, ollama.size_small, ollama.size_medium, ollama.size_large, listen.format",
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
# codex = "gpt-5.4"
# gemini = "auto"
# copilot = "claude-sonnet-4.5"

[auto]
# Settings for auto provider/model selection (-p auto / -m auto)
# provider = "claude"
# model = "haiku"

[ollama]
# Ollama-specific settings
# model = "qwen3.5"
# size = "9b"
# size_small = "2b"
# size_medium = "9b"
# size_large = "35b"

[listen]
# Default output format for listen command: "text", "json", or "rich-text"
# format = "text"
"#
        .to_string()
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
