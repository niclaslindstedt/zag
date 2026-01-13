use anyhow::{bail, Context, Result};
use std::path::PathBuf;

use super::types::Workflow;

/// Embedded workflows compiled into the binary
const EMBEDDED_SOFTWARE: &str = include_str!("../../workflows/software.json");

/// Loads workflow definitions from embedded sources or config directory.
///
/// Load order (first match wins):
/// 1. User config directory: `~/.agent/workflows/<name>.json`
/// 2. Embedded workflows compiled into the binary
pub struct WorkflowLoader {
    config_dir: PathBuf,
}

impl WorkflowLoader {
    /// Create a new loader using the default config directory (`~/.agent/workflows/`).
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        Self {
            config_dir: PathBuf::from(home).join(".agent").join("workflows"),
        }
    }

    /// Create a loader with a custom config directory.
    pub fn with_config_dir(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }

    /// Load a workflow by name.
    ///
    /// Checks the config directory first, then falls back to embedded workflows.
    pub fn load(&self, name: &str) -> Result<Workflow> {
        // 1. Check config directory override
        let config_path = self.config_dir.join(format!("{}.json", name));
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read workflow: {}", config_path.display()))?;
            return self.parse_workflow(&content, &config_path.display().to_string());
        }

        // 2. Check embedded workflows
        match name {
            "software" => self.parse_workflow(EMBEDDED_SOFTWARE, "embedded:software"),
            _ => bail!(
                "Unknown workflow: '{}'. Available workflows: {}",
                name,
                self.list_available()?.join(", ")
            ),
        }
    }

    /// List all available workflows (embedded + config directory).
    pub fn list_available(&self) -> Result<Vec<String>> {
        let mut workflows = vec!["software".to_string()]; // Embedded workflows

        // Add user-defined workflows from config directory
        if self.config_dir.exists() {
            for entry in std::fs::read_dir(&self.config_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "json") {
                    if let Some(name) = path.file_stem() {
                        let name = name.to_string_lossy().to_string();
                        if !workflows.contains(&name) {
                            workflows.push(name);
                        }
                    }
                }
            }
        }

        workflows.sort();
        Ok(workflows)
    }

    /// Get information about embedded workflows.
    pub fn get_embedded_info() -> Vec<(&'static str, &'static str)> {
        vec![
            ("software", "Epic-based software development with review loops"),
        ]
    }

    fn parse_workflow(&self, content: &str, source: &str) -> Result<Workflow> {
        serde_json::from_str(content)
            .with_context(|| format!("Failed to parse workflow from {}", source))
    }
}

impl Default for WorkflowLoader {
    fn default() -> Self {
        Self::new()
    }
}
