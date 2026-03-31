//! Watch command: event-driven reactions on session log events.
//!
//! Like `listen` but executes a shell command when specific events match.
//! Think of it as `listen` + `xargs`.

use crate::listen;
use crate::session_log::{AgentLogEvent, LogEventKind};
use anyhow::{Result, bail};
use log::debug;
use std::io::{BufRead, BufReader, Seek, SeekFrom};

/// Parameters for the watch command.
pub struct WatchParams {
    pub session_id: Option<String>,
    pub tag: Option<String>,
    pub latest: bool,
    pub on_event: String,
    pub filter_expr: Option<String>,
    pub command: Vec<String>,
    pub once: bool,
    pub json: bool,
    pub root: Option<String>,
}

/// Template variable replacement in command strings.
fn expand_template(template: &str, event: &AgentLogEvent) -> String {
    template
        .replace("{session_id}", &event.wrapper_session_id)
        .replace("{provider}", &event.provider)
        .replace("{event_type}", event_type_str(&event.kind))
        .replace("{seq}", &event.seq.to_string())
        .replace("{ts}", &event.ts)
}

fn event_type_str(kind: &LogEventKind) -> &'static str {
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
    }
}

/// Check if an event matches the filter expression (key=value pairs).
fn matches_filter(event: &AgentLogEvent, filter: &str) -> bool {
    for part in filter.split(',') {
        let part = part.trim();
        if let Some((key, value)) = part.split_once('=') {
            match key.trim() {
                "success" => {
                    if let LogEventKind::SessionEnded { success, .. } = &event.kind {
                        let expected = value.trim() == "true";
                        if *success != expected {
                            return false;
                        }
                    }
                }
                "tool_name" | "tool" => {
                    if let LogEventKind::ToolCall { tool_name, .. } = &event.kind {
                        if !tool_name
                            .to_lowercase()
                            .contains(&value.trim().to_lowercase())
                        {
                            return false;
                        }
                    }
                }
                _ => {}
            }
        }
    }
    true
}

/// Resolve sessions to watch from params.
fn resolve_watch_sessions(params: &WatchParams) -> Result<Vec<String>> {
    if let Some(ref id) = params.session_id {
        return Ok(vec![id.clone()]);
    }

    if params.latest {
        let log_path = listen::resolve_session_log(None, true, false, params.root.as_deref())?;
        // Extract session ID from path: .../sessions/<session_id>.jsonl
        let file_stem = log_path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string());
        if let Some(id) = file_stem {
            return Ok(vec![id]);
        }
    }

    if let Some(ref tag) = params.tag {
        let store = zag::session::SessionStore::load(params.root.as_deref())?;
        let tagged = store.find_by_tag(tag);
        if tagged.is_empty() {
            bail!("No sessions found with tag '{}'", tag);
        }
        return Ok(tagged.iter().map(|e| e.session_id.clone()).collect());
    }

    bail!("Specify a session ID, --latest, or --tag");
}

/// Run the watch command.
pub fn run_watch(params: WatchParams) -> Result<()> {
    if params.command.is_empty() {
        bail!("No command specified. Use -- followed by the command to execute.");
    }

    let session_ids = resolve_watch_sessions(&params)?;

    debug!(
        "Watching {} session(s) for '{}' events",
        session_ids.len(),
        params.on_event
    );

    // Watch the first session (multi-session watch would need threads)
    // In practice, orchestrators typically watch one session at a time
    if let Some(session_id) = session_ids.first() {
        let log_path =
            listen::resolve_session_log(Some(session_id), false, false, params.root.as_deref())?;

        let mut file = std::fs::File::open(&log_path)?;
        // Start from the end to only watch new events
        file.seek(SeekFrom::End(0))?;
        let mut reader = BufReader::new(file);

        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    // No new data, sleep briefly
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    continue;
                }
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let event: AgentLogEvent = match serde_json::from_str(trimmed) {
                        Ok(e) => e,
                        Err(_) => continue,
                    };

                    let event_type = event_type_str(&event.kind);
                    if event_type != params.on_event {
                        continue;
                    }

                    // Check filter
                    if let Some(ref filter) = params.filter_expr {
                        if !matches_filter(&event, filter) {
                            continue;
                        }
                    }

                    // Build and execute command
                    let expanded: Vec<String> = params
                        .command
                        .iter()
                        .map(|arg| expand_template(arg, &event))
                        .collect();

                    debug!("Watch triggered: {:?}", expanded);

                    if params.json {
                        println!("{}", serde_json::to_string(&event)?);
                    }

                    let status = std::process::Command::new(&expanded[0])
                        .args(&expanded[1..])
                        .status();

                    match status {
                        Ok(s) => debug!("Command exited: {}", s),
                        Err(e) => eprintln!("Failed to execute command: {}", e),
                    }

                    if params.once {
                        return Ok(());
                    }
                }
                Err(e) => {
                    bail!("Error reading log: {}", e);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "watch_tests.rs"]
mod tests;
