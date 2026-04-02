//! Utility functions for orchestration commands.

/// Resolve the current workspace path from an explicit root, git repo root, or cwd.
pub fn current_workspace(root: Option<&str>) -> String {
    if let Some(root) = root {
        root.to_string()
    } else if let Ok(repo_root) = zag::worktree::git_repo_root(None) {
        repo_root.to_string_lossy().to_string()
    } else {
        std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    }
}

/// Resolve the logs directory for a given project root.
pub fn logs_dir(root: Option<&str>) -> std::path::PathBuf {
    zag::config::Config::agent_dir(root).join("logs")
}

#[cfg(test)]
#[path = "util_tests.rs"]
mod tests;
