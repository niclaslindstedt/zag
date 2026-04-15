//! CLI binary pre-flight validation.
//!
//! Checks that agent CLI binaries exist in PATH before attempting to spawn
//! them, providing actionable error messages with install hints.

use anyhow::{Result, bail};
use log::debug;
use std::path::{Path, PathBuf};

/// Map an agent name to the binary it requires.
pub fn binary_for_agent(agent: &str) -> &str {
    match agent {
        "claude" => "claude",
        "codex" => "codex",
        "gemini" => "gemini",
        "copilot" => "copilot",
        "ollama" => "ollama",
        other => other,
    }
}

/// Return a human-readable install hint for an agent.
fn install_hint(agent: &str) -> &'static str {
    match agent {
        "claude" => "Install: npm install -g @anthropic-ai/claude-code",
        "codex" => "Install: npm install -g @openai/codex",
        "gemini" => "Install: npm install -g @anthropic-ai/gemini-cli",
        "copilot" => {
            "Install: npm install -g @github/copilot (see https://docs.github.com/en/copilot/concepts/agents/about-copilot-cli)"
        }
        "ollama" => "Install: https://ollama.ai/download",
        _ => "Check that the CLI is installed and available in PATH",
    }
}

/// Search for `binary_name` in the directories listed in the `PATH`
/// environment variable. Returns the first match.
fn find_in_path(binary_name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(binary_name);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

/// Check if a path points to an executable file.
fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.is_file()
            && path
                .metadata()
                .map(|m| m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

/// Verify that the CLI binary for `agent_name` is available in PATH.
///
/// Returns the resolved path on success, or an actionable error with
/// install instructions on failure.
pub fn check_binary(agent_name: &str) -> Result<PathBuf> {
    let binary = binary_for_agent(agent_name);
    debug!("Preflight check: looking for '{binary}' in PATH");

    match find_in_path(binary) {
        Some(path) => {
            debug!("Found '{}' at {}", binary, path.display());
            Ok(path)
        }
        None => {
            bail!(
                "'{}' CLI not found in PATH. {}\n\nEnsure '{}' is installed and available in your shell's PATH.",
                binary,
                install_hint(agent_name),
                binary,
            );
        }
    }
}

#[cfg(test)]
#[path = "preflight_tests.rs"]
mod tests;
