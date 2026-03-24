//! Session-to-worktree mapping store.
//!
//! Persists session-worktree mappings in `~/.agent/projects/<id>/sessions.json`
//! so that `agent run --resume <id>` can resume inside the correct workspace.

use crate::config::Config;
use crate::session_log::{GlobalSessionEntry, upsert_global_entry};
use anyhow::{Context, Result};
use chrono::{DateTime, FixedOffset};
use log::debug;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntry {
    pub session_id: String,
    pub provider: String,
    #[serde(default)]
    pub model: String,
    pub worktree_path: String,
    pub worktree_name: String,
    pub created_at: String,
    #[serde(default)]
    pub provider_session_id: Option<String>,
    #[serde(default)]
    pub sandbox_name: Option<String>,
    #[serde(default)]
    pub is_worktree: bool,
    #[serde(default)]
    pub discovered: bool,
    #[serde(default)]
    pub discovery_source: Option<String>,
    #[serde(default)]
    pub log_path: Option<String>,
    #[serde(default = "default_log_completeness")]
    pub log_completeness: String,
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

        // Also upsert entries with log_path into the global session index
        let global_dir = Config::global_base_dir();
        let project = path
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        for entry in &self.sessions {
            if let Some(ref log_path) = entry.log_path {
                let _ = upsert_global_entry(
                    &global_dir,
                    GlobalSessionEntry {
                        session_id: entry.session_id.clone(),
                        project: project.clone(),
                        log_path: log_path.clone(),
                        provider: entry.provider.clone(),
                        started_at: entry.created_at.clone(),
                    },
                );
            }
        }

        Ok(())
    }

    /// Add a session entry.
    pub fn add(&mut self, entry: SessionEntry) {
        self.sessions.retain(|existing| {
            existing.session_id != entry.session_id
                && !(entry.provider_session_id.is_some()
                    && existing.provider_session_id == entry.provider_session_id)
        });
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

    /// Find a session by provider-native session ID.
    pub fn find_by_provider_session_id(&self, id: &str) -> Option<&SessionEntry> {
        let result = self
            .sessions
            .iter()
            .find(|e| e.provider_session_id.as_deref() == Some(id));
        if result.is_some() {
            debug!("Found provider session: {}", id);
        } else {
            debug!("Provider session not found: {}", id);
        }
        result
    }

    /// Find a session by either wrapper or provider-native ID.
    pub fn find_by_any_id(&self, id: &str) -> Option<&SessionEntry> {
        self.find_by_session_id(id)
            .or_else(|| self.find_by_provider_session_id(id))
    }

    /// Get the most recently created session.
    pub fn latest(&self) -> Option<&SessionEntry> {
        self.sessions.iter().max_by(|a, b| {
            parse_created_at(&a.created_at)
                .cmp(&parse_created_at(&b.created_at))
                .then_with(|| a.session_id.cmp(&b.session_id))
        })
    }

    /// Update a wrapper session with the provider-native session ID.
    pub fn set_provider_session_id(&mut self, session_id: &str, provider_session_id: String) {
        if let Some(entry) = self
            .sessions
            .iter_mut()
            .find(|e| e.session_id == session_id)
        {
            entry.provider_session_id = Some(provider_session_id);
        }
    }

    /// Remove a session by ID.
    pub fn remove(&mut self, session_id: &str) {
        debug!("Removing session: {}", session_id);
        self.sessions.retain(|e| e.session_id != session_id);
    }

    /// List all sessions as `SessionInfo`, sorted by created_at descending (newest first).
    pub fn list(&self) -> Vec<SessionInfo> {
        let mut infos: Vec<SessionInfo> = self.sessions.iter().map(SessionInfo::from).collect();
        infos.sort_by(|a, b| {
            parse_created_at(&b.created_at)
                .cmp(&parse_created_at(&a.created_at))
                .then_with(|| b.session_id.cmp(&a.session_id))
        });
        infos
    }

    /// Get a session by any ID (wrapper or provider-native) as `SessionInfo`.
    pub fn get(&self, id: &str) -> Option<SessionInfo> {
        self.find_by_any_id(id).map(SessionInfo::from)
    }
}

/// Public session info struct for programmatic API consumers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub provider: String,
    pub model: String,
    pub created_at: String,
    pub provider_session_id: Option<String>,
    pub worktree_path: Option<String>,
    pub sandbox_name: Option<String>,
    pub log_completeness: String,
}

impl From<&SessionEntry> for SessionInfo {
    fn from(e: &SessionEntry) -> Self {
        Self {
            session_id: e.session_id.clone(),
            provider: e.provider.clone(),
            model: e.model.clone(),
            created_at: e.created_at.clone(),
            provider_session_id: e.provider_session_id.clone(),
            worktree_path: if e.worktree_path.is_empty() {
                None
            } else {
                Some(e.worktree_path.clone())
            },
            sandbox_name: e.sandbox_name.clone(),
            log_completeness: e.log_completeness.clone(),
        }
    }
}

fn default_log_completeness() -> String {
    "partial".to_string()
}

fn parse_created_at(created_at: &str) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(created_at).ok()
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
