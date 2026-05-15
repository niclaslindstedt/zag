//! Events command: structured event query API for session logs.
//!
//! Returns raw `AgentLogEvent` objects from session JSONL logs with
//! filtering by event type, sequence number, and count.

use crate::listen;
use anyhow::Result;
use std::io::{BufRead, BufReader};
use zag_agent::session_log::{AgentLogEvent, LogEventKind};

/// Parameters for the events command.
pub struct EventsParams {
    pub session_id: String,
    pub event_type: Option<String>,
    pub last: Option<usize>,
    pub after_seq: Option<u64>,
    pub before_seq: Option<u64>,
    pub count: bool,
    pub json: bool,
    pub root: Option<String>,
}

/// Map a LogEventKind to its type name string.
fn event_type_name(kind: &LogEventKind) -> &'static str {
    match kind {
        LogEventKind::SessionStarted { .. } => "session_started",
        LogEventKind::UserMessage { .. } => "user_message",
        LogEventKind::AssistantMessage { .. } => "assistant_message",
        LogEventKind::Reasoning { .. } => "reasoning",
        LogEventKind::ToolCall { .. } => "tool_call",
        LogEventKind::ToolResult { .. } => "tool_result",
        LogEventKind::Permission { .. } => "permission",
        LogEventKind::ProviderStatus { .. } => "provider_status",
        LogEventKind::Stderr { .. } => "stderr",
        LogEventKind::ParseWarning { .. } => "parse_warning",
        LogEventKind::SessionCleared { .. } => "session_cleared",
        LogEventKind::SessionEnded { .. } => "session_ended",
        LogEventKind::Heartbeat { .. } => "heartbeat",
        LogEventKind::UserEvent { .. } => "user_event",
        LogEventKind::Usage { .. } => "usage",
        LogEventKind::SessionResult { .. } => "session_result",
    }
}

/// Read and filter events from a session log.
pub fn read_events(params: &EventsParams) -> Result<Vec<AgentLogEvent>> {
    let log_path = listen::resolve_session_log(
        Some(&params.session_id),
        false,
        false,
        params.root.as_deref(),
    )?;

    let file = std::fs::File::open(&log_path)
        .map_err(|e| anyhow::anyhow!("Failed to open session log: {e}"))?;
    let reader = BufReader::new(file);

    let mut events = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let event: AgentLogEvent = match serde_json::from_str(trimmed) {
            Ok(e) => e,
            Err(_) => continue,
        };

        // Filter by sequence number
        if let Some(after) = params.after_seq {
            if event.seq <= after {
                continue;
            }
        }
        if let Some(before) = params.before_seq {
            if event.seq >= before {
                continue;
            }
        }

        // Filter by event type
        if let Some(ref type_filter) = params.event_type {
            let name = event_type_name(&event.kind);
            if name != type_filter.as_str() {
                continue;
            }
        }

        events.push(event);
    }

    // If --last N, keep only the last N events
    if let Some(n) = params.last {
        if events.len() > n {
            events = events.split_off(events.len() - n);
        }
    }

    Ok(events)
}

/// Run the events command.
pub fn run_events(params: EventsParams) -> Result<()> {
    let events = read_events(&params)?;

    if params.count {
        println!("{}", events.len());
        return Ok(());
    }

    if events.is_empty() && !params.json {
        println!("No events found.");
        return Ok(());
    }

    if params.json {
        // Output as NDJSON
        for event in &events {
            println!("{}", serde_json::to_string(event)?);
        }
    } else {
        for event in &events {
            let type_name = event_type_name(&event.kind);
            let preview = match &event.kind {
                LogEventKind::AssistantMessage { content, .. } => {
                    let preview: String = content.chars().take(120).collect();
                    format!(": {preview}")
                }
                LogEventKind::UserMessage { content, .. } => {
                    let preview: String = content.chars().take(120).collect();
                    format!(": {preview}")
                }
                LogEventKind::ToolCall { tool_name, .. } => format!(": {tool_name}"),
                LogEventKind::ToolResult {
                    tool_name, success, ..
                } => {
                    let name = tool_name.as_deref().unwrap_or("?");
                    let ok = success.map(|s| if s { "ok" } else { "err" }).unwrap_or("?");
                    format!(": {name} ({ok})")
                }
                LogEventKind::SessionEnded { success, error } => {
                    if *success {
                        ": success".to_string()
                    } else {
                        format!(
                            ": failed{}",
                            error
                                .as_deref()
                                .map(|e| format!(" - {e}"))
                                .unwrap_or_default()
                        )
                    }
                }
                _ => String::new(),
            };
            println!("[{}] seq={} {}{}", event.ts, event.seq, type_name, preview);
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "events_tests.rs"]
mod tests;
