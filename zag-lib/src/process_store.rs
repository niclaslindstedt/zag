//! Process tracking store.
//!
//! Persists process metadata in `~/.zag/processes.json` so that
//! `zag ps` can list, inspect, and kill running agent processes.

use crate::config::Config;
use anyhow::{Context, Result};
use chrono::{DateTime, FixedOffset};
use log::debug;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessEntry {
    /// UUID used as the CLI reference handle.
    pub id: String,
    /// OS PID of the zag wrapper process.
    pub pid: u32,
    /// Associated session ID (links to SessionEntry), if any.
    #[serde(default)]
    pub session_id: Option<String>,
    pub provider: String,
    pub model: String,
    /// Subcommand: "run", "exec", "review".
    pub command: String,
    /// First 100 characters of the prompt, if any.
    #[serde(default)]
    pub prompt: Option<String>,
    pub started_at: String,
    /// "running" | "exited" | "killed"
    pub status: String,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub exited_at: Option<String>,
    /// Project root path (for context).
    #[serde(default)]
    pub root: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessStore {
    pub processes: Vec<ProcessEntry>,
}

impl ProcessStore {
    fn path() -> PathBuf {
        Config::global_base_dir().join("processes.json")
    }

    /// Load process store from disk. Returns empty store if the file doesn't exist.
    pub fn load() -> Result<Self> {
        let path = Self::path();
        debug!("Loading process store from {}", path.display());
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read process store: {}", path.display()))?;
        let store: ProcessStore = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse process store: {}", path.display()))?;
        debug!("Loaded {} process entries", store.processes.len());
        Ok(store)
    }

    /// Save process store to disk.
    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }
        let content =
            serde_json::to_string_pretty(self).context("Failed to serialize process store")?;
        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write process store: {}", path.display()))?;
        debug!("Process store saved ({} entries)", self.processes.len());
        Ok(())
    }

    /// Add a new process entry, replacing any existing entry with the same id.
    pub fn add(&mut self, entry: ProcessEntry) {
        self.processes.retain(|e| e.id != entry.id);
        debug!(
            "Adding process: id={}, pid={}, provider={}",
            entry.id, entry.pid, entry.provider
        );
        self.processes.push(entry);
    }

    /// Update the status and exit metadata for a process entry.
    pub fn update_status(&mut self, id: &str, status: &str, exit_code: Option<i32>) {
        if let Some(entry) = self.processes.iter_mut().find(|e| e.id == id) {
            entry.status = status.to_string();
            entry.exit_code = exit_code;
            entry.exited_at = Some(chrono::Utc::now().to_rfc3339());
            debug!(
                "Updated process {}: status={}, exit_code={:?}",
                id, status, exit_code
            );
        }
    }

    /// Find a process entry by id.
    pub fn find(&self, id: &str) -> Option<&ProcessEntry> {
        self.processes.iter().find(|e| e.id == id)
    }

    /// List process entries sorted by started_at descending (newest first).
    pub fn list_recent(&self, limit: Option<usize>) -> Vec<&ProcessEntry> {
        let mut entries: Vec<&ProcessEntry> = self.processes.iter().collect();
        entries.sort_by(|a, b| {
            parse_started_at(&b.started_at)
                .cmp(&parse_started_at(&a.started_at))
                .then_with(|| b.id.cmp(&a.id))
        });
        if let Some(n) = limit {
            entries.truncate(n);
        }
        entries
    }
}

fn parse_started_at(s: &str) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(s).ok()
}

#[cfg(test)]
#[path = "process_store_tests.rs"]
mod tests;
