//! Session-to-worktree mapping store.
//!
//! Persists session-worktree mappings in `~/.agent/projects/<id>/sessions.json`
//! so that `agent resume <id>` can resume inside the correct worktree.

use crate::config::Config;
use anyhow::{Context, Result};
use log::debug;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntry {
    pub session_id: String,
    pub provider: String,
    pub worktree_path: String,
    pub worktree_name: String,
    pub created_at: String,
    #[serde(default)]
    pub sandbox_name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionStore {
    pub sessions: Vec<SessionEntry>,
}

impl SessionStore {
    /// Path to the sessions file.
    fn path(root: Option<&str>) -> PathBuf {
        Config::agent_dir(root).join("sessions.json")
    }

    /// Load session store from disk. Returns empty store if file doesn't exist.
    pub fn load(root: Option<&str>) -> Result<Self> {
        let path = Self::path(root);
        debug!("Loading session store from {}", path.display());
        if !path.exists() {
            debug!("Session store not found, using empty store");
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read sessions file: {}", path.display()))?;
        let store: SessionStore = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse sessions file: {}", path.display()))?;
        debug!(
            "Loaded {} sessions from {}",
            store.sessions.len(),
            path.display()
        );
        Ok(store)
    }

    /// Save session store to disk.
    pub fn save(&self, root: Option<&str>) -> Result<()> {
        let path = Self::path(root);
        debug!(
            "Saving {} sessions to {}",
            self.sessions.len(),
            path.display()
        );
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }
        let content = serde_json::to_string_pretty(self).context("Failed to serialize sessions")?;
        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write sessions file: {}", path.display()))?;
        debug!("Session store saved to {}", path.display());
        Ok(())
    }

    /// Add a session entry.
    pub fn add(&mut self, entry: SessionEntry) {
        debug!(
            "Adding session: id={}, provider={}, worktree={}",
            entry.session_id, entry.provider, entry.worktree_name
        );
        self.sessions.push(entry);
    }

    /// Find a session by ID.
    pub fn find_by_session_id(&self, id: &str) -> Option<&SessionEntry> {
        let result = self.sessions.iter().find(|e| e.session_id == id);
        if result.is_some() {
            debug!("Found session: {}", id);
        } else {
            debug!("Session not found: {}", id);
        }
        result
    }

    /// Remove a session by ID.
    pub fn remove(&mut self, session_id: &str) {
        debug!("Removing session: {}", session_id);
        self.sessions.retain(|e| e.session_id != session_id);
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
