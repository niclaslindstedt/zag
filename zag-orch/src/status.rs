//! Status command: machine-readable session health check.

use crate::listen;
use crate::ps::resolve_live_status;
use anyhow::{Result, bail};
use std::io::{BufRead, BufReader};
use zag_agent::process_store::ProcessStore;
use zag_agent::session::SessionStore;
use zag_agent::session_log::{AgentLogEvent, LogEventKind};

/// Possible session status values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Running,
    Idle,
    Completed,
    Failed,
    Dead,
    Unknown,
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "running"),
            Self::Idle => write!(f, "idle"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Dead => write!(f, "dead"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Detailed status info for JSON output.
#[derive(Debug, serde::Serialize)]
pub struct StatusInfo {
    pub session_id: String,
    pub status: SessionStatus,
    pub provider: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_heartbeat_at: Option<String>,
}

/// Determine session status by combining session store, process store, and log state.
pub fn determine_status(session_id: &str, root: Option<&str>) -> Result<StatusInfo> {
    // Look up in session store
    let session_store = SessionStore::load(root).unwrap_or_default();
    let session_entry = session_store.find_by_any_id(session_id);

    let (provider, model, name) = match session_entry {
        Some(e) => (e.provider.clone(), e.model.clone(), e.name.clone()),
        None => (String::new(), String::new(), None),
    };

    // Find in process store
    let proc_store = ProcessStore::load().unwrap_or_default();
    let proc_entry = proc_store
        .processes
        .iter()
        .filter(|e| e.session_id.as_deref() == Some(session_id))
        .max_by(|a, b| a.started_at.cmp(&b.started_at));

    // Check log for SessionEnded
    let log_path = listen::resolve_session_log(Some(session_id), false, false, root);

    if let Ok(ref path) = log_path {
        let file = std::fs::File::open(path);
        if let Ok(file) = file {
            let reader = BufReader::new(file);
            let mut last_event_ts: Option<String> = None;
            let mut last_heartbeat_ts: Option<String> = None;
            let mut ended: Option<(bool, Option<String>)> = None;

            for line in reader.lines() {
                let line = match line {
                    Ok(l) => l,
                    Err(_) => continue,
                };
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Ok(event) = serde_json::from_str::<AgentLogEvent>(trimmed) {
                    last_event_ts = Some(event.ts.clone());
                    match &event.kind {
                        LogEventKind::SessionEnded { success, error } => {
                            ended = Some((*success, error.clone()));
                        }
                        LogEventKind::Heartbeat { .. } => {
                            last_heartbeat_ts = Some(event.ts.clone());
                        }
                        _ => {}
                    }
                }
            }

            // If session ended, return completed/failed
            if let Some((success, error)) = ended {
                return Ok(StatusInfo {
                    session_id: session_id.to_string(),
                    status: if success {
                        SessionStatus::Completed
                    } else {
                        SessionStatus::Failed
                    },
                    provider,
                    model,
                    name,
                    pid: proc_entry.map(|e| e.pid),
                    error,
                    last_heartbeat_at: last_heartbeat_ts,
                });
            }

            // Session hasn't ended — check if process is alive
            if let Some(pe) = proc_entry {
                let live = resolve_live_status(pe);
                if live == "running" {
                    // Use heartbeat timestamp for liveness if available (more reliable
                    // than generic event timestamps during long thinking phases).
                    // Fall back to last event timestamp if no heartbeats yet.
                    let liveness_ts = last_heartbeat_ts.as_ref().or(last_event_ts.as_ref());
                    let status = if let Some(ts) = liveness_ts {
                        if is_recent(ts, 30) {
                            SessionStatus::Running
                        } else {
                            SessionStatus::Idle
                        }
                    } else {
                        SessionStatus::Running
                    };
                    return Ok(StatusInfo {
                        session_id: session_id.to_string(),
                        status,
                        provider,
                        model,
                        name,
                        pid: Some(pe.pid),
                        error: None,
                        last_heartbeat_at: last_heartbeat_ts
                            .as_ref()
                            .or(last_event_ts.as_ref())
                            .cloned(),
                    });
                } else {
                    // Process dead but no SessionEnded
                    return Ok(StatusInfo {
                        session_id: session_id.to_string(),
                        status: SessionStatus::Dead,
                        provider,
                        model,
                        name,
                        pid: Some(pe.pid),
                        error: Some("process died without clean exit".to_string()),
                        last_heartbeat_at: last_heartbeat_ts,
                    });
                }
            }
        }
    }

    // No log found — check process store only
    if let Some(pe) = proc_entry {
        let live = resolve_live_status(pe);
        let status = match live {
            "running" => SessionStatus::Running,
            "exited" => {
                if pe.exit_code == Some(0) {
                    SessionStatus::Completed
                } else {
                    SessionStatus::Failed
                }
            }
            "killed" => SessionStatus::Failed,
            "dead" => SessionStatus::Dead,
            _ => SessionStatus::Unknown,
        };
        return Ok(StatusInfo {
            session_id: session_id.to_string(),
            status,
            provider,
            model,
            name,
            pid: Some(pe.pid),
            error: None,
            last_heartbeat_at: None,
        });
    }

    if session_entry.is_some() {
        // Session exists in store but no process and no log — unknown
        return Ok(StatusInfo {
            session_id: session_id.to_string(),
            status: SessionStatus::Unknown,
            provider,
            model,
            name,
            pid: None,
            error: None,
            last_heartbeat_at: None,
        });
    }

    bail!("Session not found: {session_id}");
}

/// Check if an RFC3339 timestamp is within `max_age_secs` of now.
fn is_recent(ts: &str, max_age_secs: i64) -> bool {
    chrono::DateTime::parse_from_rfc3339(ts)
        .map(|dt| {
            let age = chrono::Utc::now().signed_duration_since(dt);
            age.num_seconds() < max_age_secs
        })
        .unwrap_or(false)
}

/// Run the status command.
pub fn run_status(session_id: &str, json: bool, root: Option<&str>) -> Result<()> {
    let info = determine_status(session_id, root)?;

    if json {
        println!("{}", serde_json::to_string(&info)?);
    } else {
        let status_colored = match info.status {
            SessionStatus::Running => format!("\x1b[32m{}\x1b[0m", info.status),
            SessionStatus::Idle => format!("\x1b[33m{}\x1b[0m", info.status),
            SessionStatus::Completed => format!("\x1b[32m{}\x1b[0m", info.status),
            SessionStatus::Failed | SessionStatus::Dead => {
                format!("\x1b[31m{}\x1b[0m", info.status)
            }
            SessionStatus::Unknown => format!("\x1b[2m{}\x1b[0m", info.status),
        };
        println!("{status_colored}");
    }

    Ok(())
}

#[cfg(test)]
#[path = "status_tests.rs"]
mod tests;
