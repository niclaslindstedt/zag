//! Listen command: tail a session's JSONL log file and output parsed events in real-time.

use crate::config::Config;
use crate::session_log::{AgentLogEvent, LogEventKind, SessionLogIndex};
use anyhow::{Context, Result, bail};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Output format for listen command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListenFormat {
    Text,
    Json,
    ColoredText,
}

impl ListenFormat {
    pub fn from_flags(json: bool, colors: bool, text: bool, config: &Config) -> Self {
        if json {
            return Self::Json;
        }
        if colors {
            return Self::ColoredText;
        }
        if text {
            return Self::Text;
        }
        // Check config default
        match config.listen_format() {
            Some("json") => Self::Json,
            Some("colored-text") => Self::ColoredText,
            _ => Self::Text,
        }
    }
}

/// Resolve the log file path for a session.
pub fn resolve_session_log(
    session_id: Option<&str>,
    latest: bool,
    active: bool,
    root: Option<&str>,
) -> Result<PathBuf> {
    let logs_dir = crate::session_log::logs_dir(root);
    let sessions_dir = logs_dir.join("sessions");

    if let Some(id) = session_id {
        // Try direct file path first
        let direct = sessions_dir.join(format!("{}.jsonl", id));
        if direct.exists() {
            return Ok(direct);
        }

        // Try index lookup
        let index_path = logs_dir.join("index.json");
        if index_path.exists() {
            let content = std::fs::read_to_string(&index_path)
                .with_context(|| format!("Failed to read {}", index_path.display()))?;
            let index: SessionLogIndex = serde_json::from_str(&content).unwrap_or_default();
            if let Some(entry) = index.sessions.iter().find(|e| e.wrapper_session_id == id) {
                let path = PathBuf::from(&entry.log_path);
                if path.exists() {
                    return Ok(path);
                }
            }
            // Also try matching by prefix
            let matches: Vec<_> = index
                .sessions
                .iter()
                .filter(|e| e.wrapper_session_id.starts_with(id))
                .collect();
            if matches.len() == 1 {
                let path = PathBuf::from(&matches[0].log_path);
                if path.exists() {
                    return Ok(path);
                }
            } else if matches.len() > 1 {
                bail!(
                    "Ambiguous session ID prefix '{}'. Matches: {}",
                    id,
                    matches
                        .iter()
                        .map(|e| e.wrapper_session_id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }

        bail!("No session log found for '{}'", id);
    }

    if latest {
        return resolve_latest_session(&logs_dir);
    }

    if active {
        return resolve_active_session(&sessions_dir);
    }

    bail!("Specify a session ID, --latest, or --active");
}

/// Find the latest session by `started_at` in the index.
fn resolve_latest_session(logs_dir: &Path) -> Result<PathBuf> {
    let index_path = logs_dir.join("index.json");
    if !index_path.exists() {
        bail!("No session index found. Run an agent session first.");
    }

    let content = std::fs::read_to_string(&index_path)
        .with_context(|| format!("Failed to read {}", index_path.display()))?;
    let index: SessionLogIndex = serde_json::from_str(&content).unwrap_or_default();

    if index.sessions.is_empty() {
        bail!("No sessions found in index");
    }

    let newest = index
        .sessions
        .iter()
        .max_by(|a, b| a.started_at.cmp(&b.started_at))
        .unwrap();

    let path = PathBuf::from(&newest.log_path);
    if path.exists() {
        Ok(path)
    } else {
        bail!(
            "Latest session log file no longer exists: {}",
            path.display()
        );
    }
}

/// Find the most recently modified `.jsonl` file in the sessions directory.
fn resolve_active_session(sessions_dir: &Path) -> Result<PathBuf> {
    if !sessions_dir.exists() {
        bail!("No sessions directory found. Run an agent session first.");
    }

    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    let entries = std::fs::read_dir(sessions_dir).context("Failed to read sessions directory")?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "jsonl")
            && let Ok(metadata) = entry.metadata()
            && let Ok(modified) = metadata.modified()
            && newest
                .as_ref()
                .map(|(current, _)| modified > *current)
                .unwrap_or(true)
        {
            newest = Some((modified, path));
        }
    }

    newest
        .map(|(_, path)| path)
        .ok_or_else(|| anyhow::anyhow!("No session log files found"))
}

/// Tail a session log file, printing events as they arrive.
/// Returns when a SessionEnded event is seen or the process is interrupted.
pub fn tail_session_log(path: &Path, format: ListenFormat) -> Result<()> {
    let file = File::open(path)
        .with_context(|| format!("Failed to open session log: {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line)?;

        if bytes_read > 0 {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            match format {
                ListenFormat::Json => {
                    // Pass through raw JSON
                    println!("{}", trimmed);
                }
                ListenFormat::Text | ListenFormat::ColoredText => {
                    match serde_json::from_str::<AgentLogEvent>(trimmed) {
                        Ok(event) => {
                            let formatted = if format == ListenFormat::ColoredText {
                                format_event_colored(&event)
                            } else {
                                format_event_text(&event)
                            };
                            if let Some(text) = formatted {
                                println!("{}", text);
                            }
                        }
                        Err(e) => {
                            eprintln!("[parse error] {}: {}", e, truncate(trimmed, 80));
                        }
                    }
                }
            }

            // Check if session ended
            if let Ok(event) = serde_json::from_str::<AgentLogEvent>(trimmed)
                && matches!(event.kind, LogEventKind::SessionEnded { .. })
            {
                return Ok(());
            }
        } else {
            // No new data — poll
            std::thread::sleep(std::time::Duration::from_millis(100));
            // Re-seek to current position to pick up new data appended by another process
            let pos = reader.stream_position()?;
            reader.seek(SeekFrom::Start(pos))?;
        }
    }
}

/// Format an event as plain text.
pub fn format_event_text(event: &AgentLogEvent) -> Option<String> {
    match &event.kind {
        LogEventKind::SessionStarted { command, model, .. } => {
            let model_info = model
                .as_deref()
                .map(|m| format!(" (model: {})", m))
                .unwrap_or_default();
            Some(format!("[session] Started: {}{}", command, model_info))
        }
        LogEventKind::UserMessage { content, .. } => {
            Some(format!("[user] {}", truncate(content, 200)))
        }
        LogEventKind::AssistantMessage { content, .. } => {
            Some(format!("[assistant] {}", truncate(content, 200)))
        }
        LogEventKind::Reasoning { content, .. } => {
            Some(format!("[thinking] {}", truncate(content, 200)))
        }
        LogEventKind::ToolCall {
            tool_name, input, ..
        } => {
            let input_summary = input
                .as_ref()
                .map(|v| truncate(&v.to_string(), 100))
                .unwrap_or_default();
            Some(format!("[tool] {}({})", tool_name, input_summary))
        }
        LogEventKind::ToolResult {
            tool_name,
            success,
            output,
            error,
            ..
        } => {
            let name = tool_name.as_deref().unwrap_or("unknown");
            let status = if success.unwrap_or(false) {
                "success"
            } else {
                "error"
            };
            let detail = error
                .as_deref()
                .or(output.as_deref())
                .map(|s| format!(": {}", truncate(s, 100)))
                .unwrap_or_default();
            Some(format!("[result] {}: {}{}", name, status, detail))
        }
        LogEventKind::Permission {
            tool_name, granted, ..
        } => {
            let status = if *granted { "granted" } else { "denied" };
            Some(format!("[permission] {}: {}", tool_name, status))
        }
        LogEventKind::ProviderStatus { message, .. } => {
            Some(format!("[status] {}", truncate(message, 200)))
        }
        LogEventKind::Stderr { message } => Some(format!("[stderr] {}", truncate(message, 200))),
        LogEventKind::ParseWarning { message, .. } => {
            Some(format!("[warning] {}", truncate(message, 200)))
        }
        LogEventKind::SessionEnded { success, error } => {
            let error_info = error
                .as_deref()
                .map(|e| format!(" ({})", e))
                .unwrap_or_default();
            Some(format!(
                "[session] Ended (success: {}){}",
                success, error_info
            ))
        }
    }
}

/// Format an event with ANSI colors.
pub fn format_event_colored(event: &AgentLogEvent) -> Option<String> {
    let text = format_event_text(event)?;

    // Apply colors based on event kind
    let colored = match &event.kind {
        LogEventKind::SessionStarted { .. } | LogEventKind::SessionEnded { .. } => {
            colorize(&text, "32") // green
        }
        LogEventKind::UserMessage { .. } => colorize(&text, "34"), // blue
        LogEventKind::AssistantMessage { .. } => colorize(&text, "1"), // bright/bold
        LogEventKind::Reasoning { .. } => colorize(&text, "2"),    // dim
        LogEventKind::ToolCall { .. } => colorize(&text, "33"),    // yellow
        LogEventKind::ToolResult { .. } => colorize(&text, "36"),  // cyan
        LogEventKind::Permission { .. } => colorize(&text, "35"),  // magenta
        LogEventKind::ProviderStatus { .. } => colorize(&text, "2"), // dim
        LogEventKind::Stderr { .. } => colorize(&text, "31"),      // red
        LogEventKind::ParseWarning { .. } => colorize(&text, "33;1"), // bright yellow
    };

    Some(colored)
}

fn colorize(text: &str, code: &str) -> String {
    format!("\x1b[{}m{}\x1b[0m", code, text)
}

fn truncate(s: &str, max_len: usize) -> String {
    let s = s.replace('\n', "\\n");
    if s.len() <= max_len {
        s
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
#[path = "listen_tests.rs"]
mod tests;
