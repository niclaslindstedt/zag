#[cfg(test)]
#[path = "worktree_tests.rs"]
mod tests;

use crate::config::Config;
use anyhow::{Context, Result, bail};
use log::debug;
use std::path::{Path, PathBuf};

/// Compute the base directory for worktrees: `~/.agent/worktrees/<sanitized-repo-path>/`.
pub fn worktree_base_dir(repo_root: &Path) -> PathBuf {
    let sanitized = Config::sanitize_path(&repo_root.to_string_lossy());
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".agent")
        .join("worktrees")
        .join(sanitized)
}

/// Get the git repository root from a given directory (or current directory).
pub fn git_repo_root(from: Option<&str>) -> Result<PathBuf> {
    let mut cmd = std::process::Command::new("git");
    cmd.args(["rev-parse", "--show-toplevel"]);
    if let Some(dir) = from {
        cmd.current_dir(dir);
    }
    let output = cmd
        .output()
        .context("Failed to run git rev-parse --show-toplevel")?;
    if !output.status.success() {
        bail!("--worktree requires a git repository");
    }
    let root = String::from_utf8(output.stdout)
        .context("Invalid UTF-8 in git output")?
        .trim()
        .to_string();
    Ok(PathBuf::from(root))
}

/// Generate a random worktree name like `agent-a1b2c3d4`.
pub fn generate_name() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    // Simple hash-like hex from timestamp + pid
    let hash = seed ^ (std::process::id() as u128);
    format!("agent-{:08x}", (hash & 0xFFFF_FFFF) as u32)
}

/// Create a git worktree at `~/.agent/worktrees/<sanitized-repo-path>/<name>` using detached HEAD.
/// Returns the path to the new worktree directory.
pub fn create_worktree(repo_root: &Path, name: &str) -> Result<PathBuf> {
    let worktree_path = worktree_base_dir(repo_root).join(name);

    debug!("Creating worktree at {}", worktree_path.display());

    let output = std::process::Command::new("git")
        .current_dir(repo_root)
        .args([
            "worktree",
            "add",
            worktree_path.to_str().unwrap(),
            "--detach",
        ])
        .output()
        .context("Failed to run git worktree add")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to create worktree: {}", stderr.trim());
    }

    debug!("Worktree created at {}", worktree_path.display());
    Ok(worktree_path)
}

/// Check if a worktree has any uncommitted changes (staged, unstaged, or untracked).
pub fn has_changes(path: &Path) -> Result<bool> {
    let output = std::process::Command::new("git")
        .current_dir(path)
        .args(["status", "--porcelain"])
        .output()
        .context("Failed to run git status --porcelain")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git status failed: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(!stdout.trim().is_empty())
}

/// Remove a git worktree at the given path.
pub fn remove_worktree(path: &Path) -> Result<()> {
    debug!("Removing worktree at {}", path.display());

    let output = std::process::Command::new("git")
        .args(["worktree", "remove", path.to_str().unwrap(), "--force"])
        .output()
        .context("Failed to run git worktree remove")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to remove worktree: {}", stderr.trim());
    }

    debug!("Worktree removed at {}", path.display());
    Ok(())
}
