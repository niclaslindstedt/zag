//! Subscribe command: multiplexed event stream from all active sessions.
//!
//! Watches all active session JSONL files and outputs a single merged
//! event stream. This is the read-side primitive for building real
//! orchestration on top of zag.

use crate::listen;
use crate::session_log::AgentLogEvent;
use anyhow::{Result, bail};
use log::debug;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use zag::session::SessionStore;

/// Parameters for the subscribe command.
pub struct SubscribeParams {
    pub tag: Option<String>,
    pub event_type: Option<String>,
    pub global: bool,
    pub json: bool,
    pub root: Option<String>,
}

/// A tracked log file with its current read position.
struct TrackedLog {
    #[allow(dead_code)]
    session_id: String,
    reader: BufReader<std::fs::File>,
}

/// Discover active session log files.
fn discover_sessions(params: &SubscribeParams) -> Result<Vec<(String, std::path::PathBuf)>> {
    let store = if params.global {
        SessionStore::load_all()?
    } else {
        SessionStore::load(params.root.as_deref())?
    };

    let sessions: Vec<_> = if let Some(ref tag) = params.tag {
        store.find_by_tag(tag).into_iter().cloned().collect()
    } else {
        store.sessions.clone()
    };

    let mut result = Vec::new();
    for entry in &sessions {
        match listen::resolve_session_log(
            Some(&entry.session_id),
            false,
            false,
            params.root.as_deref(),
        ) {
            Ok(path) => {
                if path.exists() {
                    result.push((entry.session_id.clone(), path));
                }
            }
            Err(_) => continue,
        }
    }

    Ok(result)
}

/// Run the subscribe command.
pub fn run_subscribe(params: SubscribeParams) -> Result<()> {
    let sessions = discover_sessions(&params)?;

    if sessions.is_empty() {
        bail!("No active sessions found to subscribe to");
    }

    debug!("Subscribing to {} session(s)", sessions.len());

    // Open all log files and seek to end
    let mut tracked: Vec<TrackedLog> = Vec::new();
    for (session_id, path) in &sessions {
        match std::fs::File::open(path) {
            Ok(mut file) => {
                let _ = file.seek(SeekFrom::End(0));
                tracked.push(TrackedLog {
                    session_id: session_id.clone(),
                    reader: BufReader::new(file),
                });
            }
            Err(e) => {
                debug!("Failed to open log for session {}: {}", session_id, e);
            }
        }
    }

    if tracked.is_empty() {
        bail!("Could not open any session logs");
    }

    // Poll loop: read new lines from all tracked logs
    loop {
        let mut had_data = false;

        for log in &mut tracked {
            loop {
                let mut line = String::new();
                match log.reader.read_line(&mut line) {
                    Ok(0) => break, // No more data in this file
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        let event: AgentLogEvent = match serde_json::from_str(trimmed) {
                            Ok(e) => e,
                            Err(_) => continue,
                        };

                        // Filter by event type
                        if let Some(ref type_filter) = params.event_type {
                            let event_type = match &event.kind {
                                crate::session_log::LogEventKind::SessionStarted { .. } => {
                                    "session_started"
                                }
                                crate::session_log::LogEventKind::SessionEnded { .. } => {
                                    "session_ended"
                                }
                                crate::session_log::LogEventKind::UserMessage { .. } => {
                                    "user_message"
                                }
                                crate::session_log::LogEventKind::AssistantMessage { .. } => {
                                    "assistant_message"
                                }
                                crate::session_log::LogEventKind::ToolCall { .. } => "tool_call",
                                crate::session_log::LogEventKind::ToolResult { .. } => {
                                    "tool_result"
                                }
                                _ => "other",
                            };
                            if event_type != type_filter.as_str() {
                                continue;
                            }
                        }

                        had_data = true;

                        if params.json {
                            println!("{}", serde_json::to_string(&event).unwrap_or_default());
                        } else {
                            let id_short =
                                &event.wrapper_session_id[..event.wrapper_session_id.len().min(8)];
                            let type_name = match &event.kind {
                                crate::session_log::LogEventKind::SessionStarted { .. } => {
                                    "session_started"
                                }
                                crate::session_log::LogEventKind::SessionEnded { .. } => {
                                    "session_ended"
                                }
                                crate::session_log::LogEventKind::AssistantMessage { .. } => {
                                    "assistant_message"
                                }
                                crate::session_log::LogEventKind::ToolCall { .. } => "tool_call",
                                _ => "event",
                            };
                            println!("[{}] {} {}", id_short, event.ts, type_name);
                        }
                    }
                    Err(_) => break,
                }
            }
        }

        if !had_data {
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    }
}

#[cfg(test)]
#[path = "subscribe_tests.rs"]
mod tests;
