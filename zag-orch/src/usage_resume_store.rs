//! Append-only persistence for in-flight scheduled resumes.
//!
//! When the foreground auto-resume loop (`run_with_auto_resume`) or the
//! interactive relay arms a wake-up timer via [`schedule_resume`], the
//! timer lives in process memory. If the process dies — crash, reboot,
//! manual kill — the timer is lost, the session log shows a
//! `UsageLimitHit` with no matching `UsageLimitResumed`, and the agent
//! never gets the resume message.
//!
//! This module records every scheduled resume to
//! `<state_dir>/scheduled_resumes.jsonl` so that:
//!
//! 1. Users can see in-flight resumes via `zag usage list`.
//! 2. Users can cancel a pending resume via `zag usage cancel <incident>`.
//! 3. A future rehydration pass can re-arm timers on relay startup.
//!
//! Format: one JSON record per line, append-only. Three record kinds:
//!
//! ```jsonl
//! {"action":"schedule","incident_id":"...","session_id":"...","provider":"...","when":"2026-...","message":"Continue","attempt":1,"log_path":"..."}
//! {"action":"complete","incident_id":"...","status":"resumed"}
//! {"action":"complete","incident_id":"...","status":"failed","error":"..."}
//! {"action":"cancel","incident_id":"..."}
//! ```
//!
//! `list_pending` reads the whole file and returns scheduled records that
//! have no matching `complete`/`cancel` tombstone.
//!
//! Concurrency: each `record_*` call performs a single `write_all` of the
//! serialized line including its trailing newline. On POSIX filesystems
//! writes shorter than `PIPE_BUF` to a file opened in `O_APPEND` mode are
//! atomic, so concurrent processes cannot interleave half-records as long
//! as no single record approaches 4 KiB. The schedule record (the largest
//! kind) is dominated by session_id / log_path / message and stays well
//! below that ceiling under normal use.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// One scheduled resume awaiting its wake-up time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PendingResume {
    pub incident_id: String,
    pub session_id: String,
    pub provider: String,
    /// Model, if known — needed to rebuild a `RespawnResumeStrategy`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Root override that was active when the resume was scheduled.
    /// `None` means the global default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    /// When the timer should fire (RFC 3339 in UTC).
    pub when: DateTime<Utc>,
    /// Message to deliver to the session on wake-up (typically `"Continue"`).
    pub message: String,
    /// Attempt counter — only meaningful within the original process's
    /// lifetime; a rehydrated resume starts a new chain at this value.
    pub attempt: u32,
    /// Path to the session log so completion events can be emitted to
    /// the right file even after the originating process is gone.
    pub log_path: PathBuf,
}

/// Outcome of a resume delivery — what `record_complete` writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompleteStatus {
    Resumed,
    Failed,
}

impl CompleteStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Resumed => "resumed",
            Self::Failed => "failed",
        }
    }
}

/// Internal wire format — one `action`-tagged record per JSONL line.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum Record {
    Schedule(PendingResume),
    Complete {
        incident_id: String,
        status: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    Cancel {
        incident_id: String,
    },
}

/// Path to the persistence file under the given root (or the global
/// `~/.zag` if `root` is `None`).
pub fn store_path(root: Option<&str>) -> PathBuf {
    zag_agent::config::Config::agent_dir(root).join("scheduled_resumes.jsonl")
}

fn append_record(root: Option<&str>, rec: &Record) -> Result<()> {
    let path = store_path(root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating parent directory for {}", path.display()))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("opening {} for append", path.display()))?;
    let mut line = serde_json::to_string(rec).context("serializing resume record")?;
    line.push('\n');
    file.write_all(line.as_bytes())
        .with_context(|| format!("writing to {}", path.display()))?;
    file.sync_data().ok();
    Ok(())
}

/// Persist a freshly-scheduled resume. Called by `schedule_resume` /
/// `run_with_auto_resume` before they hand the timer to tokio.
pub fn record_pending(root: Option<&str>, pending: &PendingResume) -> Result<()> {
    append_record(root, &Record::Schedule(pending.clone()))
}

/// Mark an incident as completed (resumed or failed) — writes a tombstone
/// so the next `list_pending` call no longer returns it.
pub fn record_complete(
    root: Option<&str>,
    incident_id: &str,
    status: CompleteStatus,
    error: Option<&str>,
) -> Result<()> {
    append_record(
        root,
        &Record::Complete {
            incident_id: incident_id.to_string(),
            status: status.as_str().to_string(),
            error: error.map(str::to_string),
        },
    )
}

/// User-requested cancel via `zag usage cancel`. Writes a tombstone so
/// rehydrating processes skip the incident.
pub fn record_cancel(root: Option<&str>, incident_id: &str) -> Result<()> {
    append_record(
        root,
        &Record::Cancel {
            incident_id: incident_id.to_string(),
        },
    )
}

/// Return all currently-pending resumes: records that have a `schedule`
/// entry but no matching `complete` or `cancel` tombstone.
pub fn list_pending(root: Option<&str>) -> Result<Vec<PendingResume>> {
    let path = store_path(root);
    list_pending_at(&path)
}

/// Version of [`list_pending`] that reads from an explicit path —
/// useful for tests and for callers that already know the file location.
pub fn list_pending_at(path: &Path) -> Result<Vec<PendingResume>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let reader = BufReader::new(&file);
    let mut scheduled: HashMap<String, PendingResume> = HashMap::new();
    let mut completed: HashSet<String> = HashSet::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        // Defensive: a partially-written line shouldn't poison the read.
        let Ok(rec) = serde_json::from_str::<Record>(&line) else {
            log::warn!("skipping malformed resume record: {line}");
            continue;
        };
        match rec {
            Record::Schedule(p) => {
                scheduled.insert(p.incident_id.clone(), p);
            }
            Record::Complete { incident_id, .. } | Record::Cancel { incident_id } => {
                completed.insert(incident_id);
            }
        }
    }
    let mut pending: Vec<PendingResume> = scheduled
        .into_iter()
        .filter(|(id, _)| !completed.contains(id))
        .map(|(_, p)| p)
        .collect();
    pending.sort_by_key(|p| p.when);
    Ok(pending)
}

#[cfg(test)]
#[path = "usage_resume_store_tests.rs"]
mod tests;
