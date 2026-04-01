//! Log command: append a custom structured event to a session's log.
//!
//! This is the write-side complement to `listen`, `subscribe`, and `events`.
//! External tools, hooks, and orchestrator scripts can annotate session timelines
//! with structured events that flow through all existing read commands.

use crate::listen;
use anyhow::{Result, bail};
use std::io::{BufRead, Write};
use zag::session_log::{AgentLogEvent, LogCompleteness, LogEventKind, LogSourceKind};

/// Parameters for the log command.
pub struct LogParams {
    pub message: String,
    pub session: Option<String>,
    pub level: String,
    pub data: Option<String>,
    pub root: Option<String>,
}

/// Resolve the target session ID from explicit flag or ZAG_SESSION_ID env var.
fn resolve_session_id(session: Option<&str>) -> Result<String> {
    if let Some(id) = session {
        return Ok(id.to_string());
    }
    if let Ok(id) = std::env::var("ZAG_SESSION_ID") {
        return Ok(id);
    }
    bail!(
        "No session specified. Use --session or run inside a zag session (ZAG_SESSION_ID env var)."
    );
}

/// Validate the log level.
fn validate_level(level: &str) -> Result<()> {
    match level {
        "info" | "warn" | "error" | "debug" => Ok(()),
        _ => bail!(
            "Invalid log level '{}'. Use: info, warn, error, debug",
            level
        ),
    }
}

/// Run the log command.
pub fn run_log(params: LogParams) -> Result<()> {
    let session_id = resolve_session_id(params.session.as_deref())?;
    validate_level(&params.level)?;

    let data: Option<serde_json::Value> = if let Some(ref data_str) = params.data {
        Some(
            serde_json::from_str(data_str)
                .map_err(|e| anyhow::anyhow!("Invalid JSON data: {}", e))?,
        )
    } else {
        None
    };

    let log_path =
        listen::resolve_session_log(Some(&session_id), false, false, params.root.as_deref())?;

    // Read existing events to determine the next sequence number and provider
    let mut max_seq: u64 = 0;
    let mut provider = String::from("unknown");

    if let Ok(file) = std::fs::File::open(&log_path) {
        let reader = std::io::BufReader::new(file);
        for line in reader.lines().map_while(Result::ok) {
            if let Ok(event) = serde_json::from_str::<AgentLogEvent>(line.trim()) {
                if event.seq > max_seq {
                    max_seq = event.seq;
                }
                provider = event.provider.clone();
            }
        }
    }

    let event = AgentLogEvent {
        seq: max_seq + 1,
        ts: chrono::Utc::now().to_rfc3339(),
        provider,
        wrapper_session_id: session_id,
        provider_session_id: None,
        source_kind: LogSourceKind::Wrapper,
        completeness: LogCompleteness::Full,
        kind: LogEventKind::UserEvent {
            level: params.level,
            message: params.message,
            data,
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

#[cfg(test)]
#[path = "log_cmd_tests.rs"]
mod tests;
