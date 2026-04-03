//! Cancel command: graceful session cancellation with clean log entry.
//!
//! Sends SIGHUP to the process and writes a SessionEnded event to the
//! session log so that `status`, `collect`, and `wait` see a clean
//! "cancelled" state.

use crate::listen;
use anyhow::{Result, bail};
use log::debug;
use std::io::Write;
use zag_agent::process_store::ProcessStore;
use zag_agent::session::SessionStore;
use zag_agent::session_log::{AgentLogEvent, LogCompleteness, LogEventKind, LogSourceKind};

/// Parameters for the cancel command.
pub struct CancelParams {
    pub session_ids: Vec<String>,
    pub tag: Option<String>,
    pub reason: Option<String>,
    pub json: bool,
    pub root: Option<String>,
}

/// Result of cancelling a single session.
#[derive(Debug, serde::Serialize)]
struct CancelResult {
    session_id: String,
    cancelled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Cancel a single session: send SIGHUP + write SessionEnded event.
fn cancel_session(session_id: &str, reason: Option<&str>, root: Option<&str>) -> CancelResult {
    let reason_msg = reason.unwrap_or("cancelled by user");

    // Find the process
    let proc_store = ProcessStore::load().unwrap_or_default();
    let proc_entry = proc_store
        .processes
        .iter()
        .filter(|e| e.session_id.as_deref() == Some(session_id))
        .max_by(|a, b| a.started_at.cmp(&b.started_at));

    // Try to send SIGHUP if process is alive
    #[cfg(unix)]
    if let Some(pe) = proc_entry {
        use nix::sys::signal::{Signal, kill};
        use nix::unistd::Pid;
        let pid = Pid::from_raw(pe.pid as i32);
        match kill(pid, Signal::SIGHUP) {
            Ok(_) => debug!("Sent SIGHUP to pid {}", pe.pid),
            Err(nix::errno::Errno::ESRCH) => {
                debug!("Process {} already dead", pe.pid);
            }
            Err(e) => {
                debug!("Failed to send SIGHUP to pid {}: {}", pe.pid, e);
            }
        }
    }
    #[cfg(not(unix))]
    let _ = proc_entry;

    // Write a SessionEnded event to the log
    let log_result = write_cancel_event(session_id, reason_msg, root);
    match log_result {
        Ok(_) => CancelResult {
            session_id: session_id.to_string(),
            cancelled: true,
            error: None,
        },
        Err(e) => CancelResult {
            session_id: session_id.to_string(),
            cancelled: false,
            error: Some(e.to_string()),
        },
    }
}

/// Write a SessionEnded event with cancellation reason to the session log.
fn write_cancel_event(session_id: &str, reason: &str, root: Option<&str>) -> Result<()> {
    let log_path = listen::resolve_session_log(Some(session_id), false, false, root)?;

    // Read existing events to determine the next sequence number
    let mut max_seq: u64 = 0;
    let mut provider = String::from("unknown");
    let mut already_ended = false;

    if let Ok(file) = std::fs::File::open(&log_path) {
        let reader = std::io::BufReader::new(file);
        for line in std::io::BufRead::lines(reader).map_while(Result::ok) {
            if let Ok(event) = serde_json::from_str::<AgentLogEvent>(line.trim()) {
                if event.seq > max_seq {
                    max_seq = event.seq;
                }
                provider = event.provider.clone();
                if matches!(event.kind, LogEventKind::SessionEnded { .. }) {
                    already_ended = true;
                }
            }
        }
    }

    if already_ended {
        debug!(
            "Session {} already has a SessionEnded event, skipping",
            session_id
        );
        return Ok(());
    }

    let event = AgentLogEvent {
        seq: max_seq + 1,
        ts: chrono::Utc::now().to_rfc3339(),
        provider,
        wrapper_session_id: session_id.to_string(),
        provider_session_id: None,
        source_kind: LogSourceKind::Wrapper,
        completeness: LogCompleteness::Full,
        kind: LogEventKind::SessionEnded {
            success: false,
            error: Some(format!("cancelled: {}", reason)),
        },
    };

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    let json = serde_json::to_string(&event)?;
    writeln!(file, "{}", json)?;

    Ok(())
}

/// Run the cancel command.
pub fn run_cancel(params: CancelParams) -> Result<()> {
    let mut session_ids = params.session_ids.clone();

    // Resolve tagged sessions
    if let Some(ref tag) = params.tag {
        let store = SessionStore::load(params.root.as_deref()).unwrap_or_default();
        let tagged = store.find_by_tag(tag);
        if tagged.is_empty() && session_ids.is_empty() {
            bail!("No sessions found with tag '{}'", tag);
        }
        for entry in tagged {
            if !session_ids.contains(&entry.session_id) {
                session_ids.push(entry.session_id.clone());
            }
        }
    }

    if session_ids.is_empty() {
        bail!("No sessions specified. Provide session IDs or --tag.");
    }

    let mut results = Vec::new();
    for id in &session_ids {
        let result = cancel_session(id, params.reason.as_deref(), params.root.as_deref());
        results.push(result);
    }

    if params.json {
        println!("{}", serde_json::to_string(&results)?);
    } else {
        for r in &results {
            if r.cancelled {
                println!("\x1b[32m\u{2713}\x1b[0m Cancelled session {}", r.session_id);
            } else {
                println!(
                    "\x1b[31m\u{2717}\x1b[0m Failed to cancel session {}: {}",
                    r.session_id,
                    r.error.as_deref().unwrap_or("unknown error")
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "cancel_tests.rs"]
mod tests;
