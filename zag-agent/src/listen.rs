//! Event formatting and filtering helpers shared by `zag listen`, the
//! library-side streaming builder options, and any consumer that wants to
//! render `AgentLogEvent` values.
//!
//! The pure formatting helpers live here (not in `zag-orch`) so that
//! `zag-agent::AgentBuilder` can reference them without introducing a
//! dependency cycle — `zag-orch` already depends on `zag-agent`.

use crate::config::Config;
use crate::session_log::{AgentLogEvent, LogEventKind};
use anyhow::Result;
use chrono::{DateTime, Local};

/// Output format for the listen-style event stream.
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
        match config.listen_format() {
            Some("json") => Self::Json,
            Some("rich-text") => Self::RichText,
            _ => Self::Text,
        }
    }
}

/// Format an RFC3339 timestamp string using a strftime-style format, converted to local time.
pub fn format_ts(ts: &str, fmt: &str) -> String {
    DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.with_timezone(&Local).format(fmt).to_string())
        .unwrap_or_else(|_| ts[..ts.len().min(19)].to_string())
}

/// Prepend a timestamp prefix to a formatted event string, preserving any leading newline.
pub fn with_timestamp(ts_str: &str, text: &str) -> String {
    if let Some(rest) = text.strip_prefix('\n') {
        format!("\n[{ts_str}] {rest}")
    } else {
        format!("[{ts_str}] {text}")
    }
}

/// Event-kind name used for filter matching.
pub fn event_type_name(kind: &LogEventKind) -> &'static str {
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
        LogEventKind::UsageLimitHit { .. } => "usage_limit_hit",
        LogEventKind::UsageLimitResumed { .. } => "usage_limit_resumed",
        LogEventKind::UsageLimitResumeFailed { .. } => "usage_limit_resume_failed",
    }
}

/// Check if an event matches the filter set.
pub fn matches_filter(kind: &LogEventKind, filters: Option<&[String]>) -> bool {
    match filters {
        None => true,
        Some(f) => f.iter().any(|filter| filter == event_type_name(kind)),
    }
}

/// Format an event as plain text with styled prefixes.
pub fn format_event_text(event: &AgentLogEvent, show_thinking: bool) -> Option<String> {
    match &event.kind {
        LogEventKind::SessionStarted { command, model, .. } => {
            let model_info = model
                .as_deref()
                .map(|m| format!(" (model: {m})"))
                .unwrap_or_default();
            Some(format!("\n\u{25cf} Started: {command}{model_info}"))
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
            Some(format!("\n  \u{26a1} {tool_name}{summary}"))
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
                Some(format!("  \u{2713}{detail}"))
            } else {
                let detail = output
                    .as_deref()
                    .map(|s| format!(" {}", format_tool_output(s)))
                    .unwrap_or_default();
                Some(format!("  \u{2717}{detail}"))
            }
        }
        LogEventKind::Permission {
            tool_name, granted, ..
        } => {
            let icon = if *granted { "\u{1f513}" } else { "\u{1f512}" };
            Some(format!("  {icon} {tool_name}"))
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
                .map(|e| format!(": {e}"))
                .unwrap_or_default();
            Some(format!("\n\u{25cf} Session {status}{error_info}"))
        }
        LogEventKind::Heartbeat { .. } => None,
        LogEventKind::Usage {
            input_tokens,
            output_tokens,
            total_cost_usd,
            ..
        } => {
            let cost_str = total_cost_usd
                .map(|c| format!(", cost=${c:.4}"))
                .unwrap_or_default();
            Some(format!(
                "  tokens: {input_tokens} in / {output_tokens} out{cost_str}"
            ))
        }
        LogEventKind::UserEvent { level, message, .. } => {
            Some(format!("  [{}] {}", level, truncate(message, 200)))
        }
        LogEventKind::SessionResult { result } => {
            Some(format!("\n\u{25cf} Result: {}", truncate(result, 200)))
        }
        LogEventKind::UsageLimitHit {
            provider,
            scope,
            reset_at,
            scheduled_resume_at,
            fallback_used,
            ..
        } => {
            let resets = reset_at
                .as_deref()
                .map(|t| format!(" — resets {t}"))
                .unwrap_or_default();
            let sched = scheduled_resume_at
                .as_deref()
                .map(|t| format!(", resuming {t}"))
                .unwrap_or_default();
            let fb = if *fallback_used { " (fallback)" } else { "" };
            Some(format!(
                "\n\u{26a0} {provider} usage limit ({scope}){resets}{sched}{fb}"
            ))
        }
        LogEventKind::UsageLimitResumed {
            resume_message,
            attempt,
            ..
        } => Some(format!(
            "\n\u{21bb} Resumed (attempt {attempt}): {}",
            truncate(resume_message, 80)
        )),
        LogEventKind::UsageLimitResumeFailed { error, attempt, .. } => Some(format!(
            "\n\u{2717} Resume failed (attempt {attempt}): {}",
            truncate(error, 200)
        )),
    }
}

/// Format an event with ANSI rich text (colors, bold, dim, italic).
pub fn format_event_rich(event: &AgentLogEvent, show_thinking: bool) -> Option<String> {
    match &event.kind {
        LogEventKind::SessionStarted { command, model, .. } => {
            let model_info = model
                .as_deref()
                .map(|m| format!(" \x1b[2m(model: {m})\x1b[0m"))
                .unwrap_or_default();
            Some(format!(
                "\n\x1b[32m\u{25cf}\x1b[0m Started: \x1b[1m{command}\x1b[0m{model_info}"
            ))
        }
        LogEventKind::UserMessage { content, .. } => Some(format!(
            "\n\x1b[34m\u{276f}\x1b[0m \x1b[1m{}\x1b[0m",
            render_content(content)
        )),
        LogEventKind::AssistantMessage { content, .. } => {
            let rendered = render_markdown(content.trim());
            let indented = indent_continuation(&rendered, "  ");
            Some(format!("\n\x1b[1m\u{23fa}\x1b[0m {indented}"))
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
            Some(format!("\n  \x1b[33m\u{26a1} {tool_name}\x1b[0m{summary}"))
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
                Some(format!("  \x1b[32m\u{2713}\x1b[0m{detail}"))
            } else {
                let detail = output
                    .as_deref()
                    .map(|s| format!(" \x1b[2m{}\x1b[0m", format_tool_output(s)))
                    .unwrap_or_default();
                Some(format!("  \x1b[31m\u{2717}\x1b[0m{detail}"))
            }
        }
        LogEventKind::Permission {
            tool_name, granted, ..
        } => {
            if *granted {
                Some(format!(
                    "  \x1b[32m\u{1f513}\x1b[0m \x1b[2m{tool_name}\x1b[0m"
                ))
            } else {
                Some(format!(
                    "  \x1b[31m\u{1f512}\x1b[0m \x1b[2m{tool_name}\x1b[0m"
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
                .map(|e| format!(": {e}"))
                .unwrap_or_default();
            Some(format!(
                "\n\x1b[{color}m\u{25cf}\x1b[0m Session {status}{error_info}"
            ))
        }
        LogEventKind::Heartbeat { .. } => None,
        LogEventKind::Usage {
            input_tokens,
            output_tokens,
            total_cost_usd,
            ..
        } => {
            let cost_str = total_cost_usd
                .map(|c| format!(", cost=\x1b[33m${c:.4}\x1b[0m"))
                .unwrap_or_default();
            Some(format!(
                "  \x1b[2mtokens: {input_tokens} in / {output_tokens} out{cost_str}\x1b[0m"
            ))
        }
        LogEventKind::UserEvent { level, message, .. } => {
            let color = match level.as_str() {
                "error" => "31",
                "warn" => "33",
                _ => "36",
            };
            Some(format!(
                "  \x1b[{}m[{}]\x1b[0m {}",
                color,
                level,
                truncate(message, 200)
            ))
        }
        LogEventKind::SessionResult { result } => Some(format!(
            "\n\x1b[32m\u{25cf}\x1b[0m Result: \x1b[1m{}\x1b[0m",
            truncate(result, 200)
        )),
        LogEventKind::UsageLimitHit {
            provider,
            scope,
            reset_at,
            scheduled_resume_at,
            fallback_used,
            ..
        } => {
            let resets = reset_at
                .as_deref()
                .map(|t| format!(" — resets \x1b[1m{t}\x1b[0m"))
                .unwrap_or_default();
            let sched = scheduled_resume_at
                .as_deref()
                .map(|t| format!(", \x1b[2mresuming {t}\x1b[0m"))
                .unwrap_or_default();
            let fb = if *fallback_used {
                " \x1b[2m(fallback)\x1b[0m"
            } else {
                ""
            };
            Some(format!(
                "\n\x1b[33m\u{26a0}\x1b[0m \x1b[1m{provider}\x1b[0m usage limit \x1b[2m({scope})\x1b[0m{resets}{sched}{fb}"
            ))
        }
        LogEventKind::UsageLimitResumed {
            resume_message,
            attempt,
            ..
        } => Some(format!(
            "\n\x1b[32m\u{21bb}\x1b[0m Resumed \x1b[2m(attempt {attempt})\x1b[0m: \x1b[1m{}\x1b[0m",
            truncate(resume_message, 80)
        )),
        LogEventKind::UsageLimitResumeFailed { error, attempt, .. } => Some(format!(
            "\n\x1b[31m\u{2717}\x1b[0m Resume failed \x1b[2m(attempt {attempt})\x1b[0m: {}",
            truncate(error, 200)
        )),
    }
}

/// Format an event for a chosen `ListenFormat` (Json is passed through as
/// compact JSON by `serde_json`). Returns `None` if the event is
/// intentionally suppressed (e.g. heartbeats in text mode).
pub fn format_event(
    event: &AgentLogEvent,
    format: ListenFormat,
    show_thinking: bool,
) -> Option<String> {
    match format {
        ListenFormat::Json => serde_json::to_string(event).ok(),
        ListenFormat::Text => format_event_text(event, show_thinking),
        ListenFormat::RichText => format_event_rich(event, show_thinking),
    }
}

/// Guess a listen format from a config-style string value, used when
/// wiring library callers from config files.
pub fn parse_listen_format(value: Option<&str>) -> Result<Option<ListenFormat>> {
    Ok(match value {
        None => None,
        Some("json") => Some(ListenFormat::Json),
        Some("rich-text") => Some(ListenFormat::RichText),
        Some("text") => Some(ListenFormat::Text),
        Some(other) => anyhow::bail!(
            "Unknown listen format '{other}' — expected one of: json, rich-text, text"
        ),
    })
}

fn summarize_tool_input(_tool_name: &str, input: Option<&serde_json::Value>) -> String {
    let Some(input) = input else {
        return String::new();
    };
    let obj = match input.as_object() {
        Some(o) => o,
        None => return String::new(),
    };

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
        return format!(": {p}{desc}");
    }

    let json = input.to_string();
    if json.len() > 2 {
        format!("({})", truncate(&json, 80))
    } else {
        String::new()
    }
}

fn shorten_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() <= 3 {
        path.to_string()
    } else {
        format!(".../{}", parts[parts.len() - 3..].join("/"))
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

fn format_tool_output(s: &str) -> String {
    indent_continuation(s.trim(), "    ")
}

fn render_content(s: &str) -> String {
    s.trim().to_string()
}

fn render_markdown(s: &str) -> String {
    let text = termimad::text(s);
    format!("{text}").trim_end().to_string()
}

fn indent_continuation(s: &str, prefix: &str) -> String {
    let mut lines = s.lines();
    let first = lines.next().unwrap_or("");
    let rest: Vec<String> = lines.map(|l| format!("{prefix}{l}")).collect();
    if rest.is_empty() {
        first.to_string()
    } else {
        format!("{}\n{}", first, rest.join("\n"))
    }
}

#[cfg(test)]
#[path = "listen_tests.rs"]
mod tests;
