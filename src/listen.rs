//! Listen command: tail a session's JSONL log file and output parsed events in real-time.

use crate::config::Config;
use crate::session_log::{AgentLogEvent, LogEventKind, SessionLogIndex};
use anyhow::{Context, Result, bail};
use chrono::{DateTime, Local};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use zag::process_store::ProcessStore;
use zag::session_log::load_global_index;

/// Output format for listen command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListenFormat {
    Text,
    Json,
    RichText,
}

impl ListenFormat {
    pub fn from_flags(json: bool, rich_text: bool, text: bool, config: &Config) -> Self {
        if json {
            return Self::Json;
        }
        if rich_text {
            return Self::RichText;
        }
        if text {
            return Self::Text;
        }
        // Check config default
        match config.listen_format() {
            Some("json") => Self::Json,
            Some("rich-text") => Self::RichText,
            _ => Self::Text,
        }
    }
}

/// Resolve a session ID from a process reference (OS PID or zag process UUID).
///
/// Accepts either a numeric OS PID or a zag process UUID (the `id` field).
/// If multiple entries match the same PID (OS PIDs are recycled), the most
/// recently started process is used.
pub fn resolve_session_from_ps(value: &str) -> Result<String> {
    let store = ProcessStore::load().context("Failed to load process store")?;

    // Try to match by zag UUID first (exact string match on `id`)
    let by_id: Vec<_> = store
        .processes
        .iter()
        .filter(|e| e.id == value || e.id.starts_with(value))
        .collect();

    let entry = if !by_id.is_empty() {
        // Pick the most recent among UUID matches
        by_id
            .into_iter()
            .max_by(|a, b| a.started_at.cmp(&b.started_at))
    } else {
        // Try numeric OS PID
        let pid: u32 = value
            .parse()
            .with_context(|| format!("'{}' is not a valid PID or process UUID", value))?;
        let by_pid: Vec<_> = store.processes.iter().filter(|e| e.pid == pid).collect();
        by_pid
            .into_iter()
            .max_by(|a, b| a.started_at.cmp(&b.started_at))
    };

    let entry = entry.ok_or_else(|| anyhow::anyhow!("No process found for '{}'", value))?;

    entry.session_id.clone().ok_or_else(|| {
        anyhow::anyhow!(
            "Process '{}' (pid {}) has no associated session",
            entry.id,
            entry.pid
        )
    })
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

        // Fallback: check global session index
        if let Some(path) = lookup_global_index_by_id(id) {
            return Ok(path);
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

    // Try project-scoped index first
    if index_path.exists() {
        let content = std::fs::read_to_string(&index_path)
            .with_context(|| format!("Failed to read {}", index_path.display()))?;
        let index: SessionLogIndex = serde_json::from_str(&content).unwrap_or_default();

        if let Some(newest) = index
            .sessions
            .iter()
            .max_by(|a, b| a.started_at.cmp(&b.started_at))
        {
            let path = PathBuf::from(&newest.log_path);
            if path.exists() {
                return Ok(path);
            }
        }
    }

    // Fallback: check global session index for the latest across all projects
    let global_dir = Config::global_base_dir();
    if let Ok(global_index) = load_global_index(&global_dir)
        && let Some(newest) = global_index
            .sessions
            .iter()
            .max_by(|a, b| a.started_at.cmp(&b.started_at))
    {
        let path = PathBuf::from(&newest.log_path);
        if path.exists() {
            return Ok(path);
        }
    }

    bail!("No session index found. Run an agent session first.");
}

/// Look up a session ID (exact or prefix) in the global session index.
fn lookup_global_index_by_id(id: &str) -> Option<PathBuf> {
    let global_dir = Config::global_base_dir();
    let global_index = load_global_index(&global_dir).ok()?;

    // Exact match
    if let Some(entry) = global_index.sessions.iter().find(|e| e.session_id == id) {
        let path = PathBuf::from(&entry.log_path);
        if path.exists() {
            return Some(path);
        }
    }

    // Prefix match
    let matches: Vec<_> = global_index
        .sessions
        .iter()
        .filter(|e| e.session_id.starts_with(id))
        .collect();
    if matches.len() == 1 {
        let path = PathBuf::from(&matches[0].log_path);
        if path.exists() {
            return Some(path);
        }
    }

    None
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

/// Format an RFC3339 timestamp string using a strftime-style format, converted to local time.
fn format_ts(ts: &str, fmt: &str) -> String {
    DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.with_timezone(&Local).format(fmt).to_string())
        .unwrap_or_else(|_| ts[..ts.len().min(19)].to_string())
}

/// Prepend a timestamp prefix to a formatted event string, preserving any leading newline.
fn with_timestamp(ts_str: &str, text: &str) -> String {
    if let Some(rest) = text.strip_prefix('\n') {
        format!("\n[{}] {}", ts_str, rest)
    } else {
        format!("[{}] {}", ts_str, text)
    }
}

/// Get the event type name for filtering (matches LogEventKind variant names in snake_case).
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
    }
}

/// Check if an event matches the filter set.
fn matches_filter(kind: &LogEventKind, filters: Option<&[String]>) -> bool {
    match filters {
        None => true,
        Some(f) => f.iter().any(|filter| filter == event_type_name(kind)),
    }
}

/// Tail a session log file, printing events as they arrive.
/// Returns when a SessionEnded event is seen or the process is interrupted.
pub fn tail_session_log(
    path: &Path,
    format: ListenFormat,
    show_thinking: bool,
    show_timestamps: bool,
    config: &Config,
    filters: Option<&[String]>,
) -> Result<()> {
    let ts_fmt = config.listen_timestamp_format().to_string();
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
                    // For JSON, parse to check filter, then pass through
                    if let Ok(event) = serde_json::from_str::<AgentLogEvent>(trimmed) {
                        if matches_filter(&event.kind, filters) {
                            println!("{}", trimmed);
                        }
                    } else {
                        println!("{}", trimmed);
                    }
                }
                ListenFormat::Text | ListenFormat::RichText => {
                    match serde_json::from_str::<AgentLogEvent>(trimmed) {
                        Ok(event) => {
                            if !matches_filter(&event.kind, filters) {
                                // Check if session ended even when filtered out
                                if matches!(event.kind, LogEventKind::SessionEnded { .. }) {
                                    return Ok(());
                                }
                                continue;
                            }
                            let formatted = if format == ListenFormat::RichText {
                                format_event_rich(&event, show_thinking)
                            } else {
                                format_event_text(&event, show_thinking)
                            };
                            if let Some(text) = formatted {
                                if show_timestamps {
                                    let ts_str = if format == ListenFormat::RichText {
                                        format!("\x1b[2m[{}]\x1b[0m", format_ts(&event.ts, &ts_fmt))
                                    } else {
                                        format!("[{}]", format_ts(&event.ts, &ts_fmt))
                                    };
                                    println!("{}", with_timestamp(&ts_str, &text));
                                } else {
                                    println!("{}", text);
                                }
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

/// Format an event as plain text with styled prefixes.
pub fn format_event_text(event: &AgentLogEvent, show_thinking: bool) -> Option<String> {
    match &event.kind {
        LogEventKind::SessionStarted { command, model, .. } => {
            let model_info = model
                .as_deref()
                .map(|m| format!(" (model: {})", m))
                .unwrap_or_default();
            Some(format!("\n\u{25cf} Started: {}{}", command, model_info))
        }
        LogEventKind::UserMessage { content, .. } => {
            Some(format!("\n\u{276f} {}", render_content(content)))
        }
        LogEventKind::AssistantMessage { content, .. } => Some(format!(
            "\n\u{23fa} {}",
            indent_continuation(&render_content(content), "  ")
        )),
        LogEventKind::Reasoning { content, .. } => {
            if !show_thinking {
                return None;
            }
            Some(format!(
                "\n  \u{2026} {}\n",
                indent_continuation(&render_content(content), "    ")
            ))
        }
        LogEventKind::ToolCall {
            tool_name, input, ..
        } => {
            let summary = summarize_tool_input(tool_name, input.as_ref());
            Some(format!("\n  \u{26a1} {}{}", tool_name, summary))
        }
        LogEventKind::ToolResult {
            success,
            output,
            error,
            ..
        } => {
            if let Some(err) = error.as_deref() {
                Some(format!("  \u{2717} {}", format_tool_output(err)))
            } else if success.unwrap_or(false) {
                let detail = output
                    .as_deref()
                    .map(|s| format!(" {}", format_tool_output(s)))
                    .unwrap_or_default();
                Some(format!("  \u{2713}{}", detail))
            } else {
                let detail = output
                    .as_deref()
                    .map(|s| format!(" {}", format_tool_output(s)))
                    .unwrap_or_default();
                Some(format!("  \u{2717}{}", detail))
            }
        }
        LogEventKind::Permission {
            tool_name, granted, ..
        } => {
            let icon = if *granted { "\u{1f513}" } else { "\u{1f512}" };
            Some(format!("  {} {}", icon, tool_name))
        }
        LogEventKind::ProviderStatus { message, .. } => {
            Some(format!("  > {}", truncate(message, 200)))
        }
        LogEventKind::Stderr { message } => Some(format!("  ! {}", truncate(message, 200))),
        LogEventKind::ParseWarning { message, .. } => {
            Some(format!("  ? {}", truncate(message, 200)))
        }
        LogEventKind::SessionCleared {
            old_session_id,
            new_session_id,
        } => {
            let old = old_session_id.as_deref().unwrap_or("unknown");
            let new = new_session_id.as_deref().unwrap_or("pending");
            Some(format!(
                "\n\u{25cf} Session cleared (old: {}, new: {})",
                truncate(old, 36),
                truncate(new, 36)
            ))
        }
        LogEventKind::SessionEnded { success, error } => {
            let status = if *success { "completed" } else { "failed" };
            let error_info = error
                .as_deref()
                .map(|e| format!(": {}", e))
                .unwrap_or_default();
            Some(format!("\n\u{25cf} Session {}{}", status, error_info))
        }
        LogEventKind::Heartbeat { .. } => None,
    }
}

/// Summarize tool input into a readable short form.
/// Uses well-known JSON key names (provider-agnostic) to extract a human-readable summary.
fn summarize_tool_input(_tool_name: &str, input: Option<&serde_json::Value>) -> String {
    let Some(input) = input else {
        return String::new();
    };
    let obj = match input.as_object() {
        Some(o) => o,
        None => return String::new(),
    };

    // Well-known keys that tend to be descriptive across providers, ordered by priority.
    // First match wins as the primary summary; a secondary "description" is appended if present.
    const SUMMARY_KEYS: &[&str] = &[
        "command",
        "file_path",
        "path",
        "pattern",
        "query",
        "url",
        "script",
        "content",
    ];

    let mut primary: Option<String> = None;
    for key in SUMMARY_KEYS {
        if let Some(val) = obj.get(*key).and_then(|v| v.as_str()) {
            let display = if *key == "file_path" || *key == "path" {
                shorten_path(val)
            } else {
                truncate(val, 80)
            };
            primary = Some(display);
            break;
        }
    }

    if let Some(p) = primary {
        let desc = obj
            .get("description")
            .and_then(|v| v.as_str())
            .map(|d| format!(" — {}", truncate(d, 60)))
            .unwrap_or_default();
        return format!(": {}{}", p, desc);
    }

    // Fallback: compact JSON
    let json = input.to_string();
    if json.len() > 2 {
        format!("({})", truncate(&json, 80))
    } else {
        String::new()
    }
}

/// Shorten a file path by keeping only the last 2-3 components.
fn shorten_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() <= 3 {
        path.to_string()
    } else {
        format!(".../{}", parts[parts.len() - 3..].join("/"))
    }
}

/// Format an event with ANSI rich text (colors, bold, dim, italic).
pub fn format_event_rich(event: &AgentLogEvent, show_thinking: bool) -> Option<String> {
    match &event.kind {
        LogEventKind::SessionStarted { command, model, .. } => {
            let model_info = model
                .as_deref()
                .map(|m| format!(" \x1b[2m(model: {})\x1b[0m", m))
                .unwrap_or_default();
            Some(format!(
                "\n\x1b[32m\u{25cf}\x1b[0m Started: \x1b[1m{}\x1b[0m{}",
                command, model_info
            ))
        }
        LogEventKind::UserMessage { content, .. } => Some(format!(
            "\n\x1b[34m\u{276f}\x1b[0m \x1b[1m{}\x1b[0m",
            render_content(content)
        )),
        LogEventKind::AssistantMessage { content, .. } => {
            let rendered = render_markdown(content.trim());
            let indented = indent_continuation(&rendered, "  ");
            Some(format!("\n\x1b[1m\u{23fa}\x1b[0m {}", indented))
        }
        LogEventKind::Reasoning { content, .. } => {
            if !show_thinking {
                return None;
            }
            Some(format!(
                "\n  \x1b[2;3m\u{2026} {}\x1b[0m\n",
                indent_continuation(&render_content(content), "    ")
            ))
        }
        LogEventKind::ToolCall {
            tool_name, input, ..
        } => {
            let summary = summarize_tool_input(tool_name, input.as_ref());
            Some(format!(
                "\n  \x1b[33m\u{26a1} {}\x1b[0m{}",
                tool_name, summary
            ))
        }
        LogEventKind::ToolResult {
            success,
            output,
            error,
            ..
        } => {
            if let Some(err) = error.as_deref() {
                Some(format!(
                    "  \x1b[31m\u{2717}\x1b[0m \x1b[2m{}\x1b[0m",
                    format_tool_output(err)
                ))
            } else if success.unwrap_or(false) {
                let detail = output
                    .as_deref()
                    .map(|s| format!(" \x1b[2m{}\x1b[0m", format_tool_output(s)))
                    .unwrap_or_default();
                Some(format!("  \x1b[32m\u{2713}\x1b[0m{}", detail))
            } else {
                let detail = output
                    .as_deref()
                    .map(|s| format!(" \x1b[2m{}\x1b[0m", format_tool_output(s)))
                    .unwrap_or_default();
                Some(format!("  \x1b[31m\u{2717}\x1b[0m{}", detail))
            }
        }
        LogEventKind::Permission {
            tool_name, granted, ..
        } => {
            if *granted {
                Some(format!(
                    "  \x1b[32m\u{1f513}\x1b[0m \x1b[2m{}\x1b[0m",
                    tool_name
                ))
            } else {
                Some(format!(
                    "  \x1b[31m\u{1f512}\x1b[0m \x1b[2m{}\x1b[0m",
                    tool_name
                ))
            }
        }
        LogEventKind::ProviderStatus { message, .. } => {
            Some(format!("  \x1b[2m> {}\x1b[0m", truncate(message, 200)))
        }
        LogEventKind::Stderr { message } => Some(format!(
            "  \x1b[31m!\x1b[0m \x1b[2m{}\x1b[0m",
            truncate(message, 200)
        )),
        LogEventKind::ParseWarning { message, .. } => Some(format!(
            "  \x1b[33m?\x1b[0m \x1b[2m{}\x1b[0m",
            truncate(message, 200)
        )),
        LogEventKind::SessionCleared {
            old_session_id,
            new_session_id,
        } => {
            let old = old_session_id.as_deref().unwrap_or("unknown");
            let new = new_session_id.as_deref().unwrap_or("pending");
            Some(format!(
                "\n\x1b[33m\u{25cf}\x1b[0m Session cleared \x1b[2m(old: {}, new: {})\x1b[0m",
                truncate(old, 36),
                truncate(new, 36)
            ))
        }
        LogEventKind::SessionEnded { success, error } => {
            let (status, color) = if *success {
                ("completed", "32")
            } else {
                ("failed", "31")
            };
            let error_info = error
                .as_deref()
                .map(|e| format!(": {}", e))
                .unwrap_or_default();
            Some(format!(
                "\n\x1b[{}m\u{25cf}\x1b[0m Session {}{}",
                color, status, error_info
            ))
        }
        LogEventKind::Heartbeat { .. } => None,
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    let s = s.replace('\n', "\\n");
    if s.len() <= max_len {
        s
    } else {
        format!("{}...", &s[..max_len])
    }
}

/// Format tool output, keeping real newlines (indented).
fn format_tool_output(s: &str) -> String {
    indent_continuation(s.trim(), "    ")
}

/// Render content for display: preserve newlines, trim leading/trailing whitespace.
fn render_content(s: &str) -> String {
    s.trim().to_string()
}

/// Render markdown content as ANSI-styled terminal text using termimad.
fn render_markdown(s: &str) -> String {
    let text = termimad::text(s);
    // termimad::text returns a FmtText whose Display impl produces ANSI output.
    // Trim trailing whitespace/newlines that termimad may add.
    format!("{}", text).trim_end().to_string()
}

/// Indent continuation lines (2nd line onwards) with the given prefix.
fn indent_continuation(s: &str, prefix: &str) -> String {
    let mut lines = s.lines();
    let first = lines.next().unwrap_or("");
    let rest: Vec<String> = lines.map(|l| format!("{}{}", prefix, l)).collect();
    if rest.is_empty() {
        first.to_string()
    } else {
        format!("{}\n{}", first, rest.join("\n"))
    }
}

#[cfg(test)]
#[path = "listen_tests.rs"]
mod tests;
