//! Summary command: log-based session summarization and stats.
//!
//! Reads the JSONL session log and produces a structured summary of what
//! happened: tool usage, file changes, duration, turn count, and result.
//! No LLM call — purely log-based introspection.

use crate::listen;
use anyhow::{Result, bail};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use zag_agent::session::SessionStore;
use zag_agent::session_log::{AgentLogEvent, LogEventKind};

/// Parameters for the summary command.
pub struct SummaryParams {
    pub session_ids: Vec<String>,
    pub tag: Option<String>,
    pub stats: bool,
    pub json: bool,
    pub root: Option<String>,
}

/// Summary of a single session.
#[derive(Debug, serde::Serialize)]
pub struct SessionSummary {
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub provider: String,
    pub model: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_secs: Option<f64>,
    pub turns: u32,
    pub tool_calls: HashMap<String, u32>,
    pub total_tool_calls: u32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub files_modified: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub event_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_cost_usd: Option<f64>,
}

/// Build a summary from a session's JSONL log.
fn summarize_session(session_id: &str, root: Option<&str>) -> Result<SessionSummary> {
    let store = SessionStore::load(root).unwrap_or_default();
    let entry = store.find_by_any_id(session_id);
    let (provider, model, name) = match entry {
        Some(e) => (e.provider.clone(), e.model.clone(), e.name.clone()),
        None => (String::new(), String::new(), None),
    };

    let log_path = listen::resolve_session_log(Some(session_id), false, false, root)?;
    let file = std::fs::File::open(&log_path)
        .map_err(|e| anyhow::anyhow!("Failed to open session log: {}", e))?;
    let reader = BufReader::new(file);

    let mut tool_calls: HashMap<String, u32> = HashMap::new();
    let mut files_modified: Vec<String> = Vec::new();
    let mut first_ts: Option<String> = None;
    let mut last_ts: Option<String> = None;
    let mut turns: u32 = 0;
    let mut last_assistant_msg: Option<String> = None;
    let mut status = "running".to_string();
    let mut error: Option<String> = None;
    let mut event_count: u32 = 0;
    let mut total_input_tokens: u64 = 0;
    let mut total_output_tokens: u64 = 0;
    let mut total_cost: f64 = 0.0;
    let mut has_usage = false;

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

        event_count += 1;
        if first_ts.is_none() {
            first_ts = Some(event.ts.clone());
        }
        last_ts = Some(event.ts.clone());

        match &event.kind {
            LogEventKind::AssistantMessage { content, .. } => {
                turns += 1;
                last_assistant_msg = Some(content.clone());
            }
            LogEventKind::ToolCall {
                tool_name, input, ..
            } => {
                *tool_calls.entry(tool_name.clone()).or_insert(0) += 1;

                // Try to extract file paths from tool input
                if let Some(input) = input {
                    if let Some(path) = input.get("file_path").and_then(|v| v.as_str()) {
                        if !files_modified.contains(&path.to_string()) {
                            files_modified.push(path.to_string());
                        }
                    }
                }
            }
            LogEventKind::SessionEnded {
                success,
                error: err,
            } => {
                status = if *success {
                    "completed".to_string()
                } else {
                    "failed".to_string()
                };
                error = err.clone();
            }
            LogEventKind::Usage {
                input_tokens,
                output_tokens,
                total_cost_usd,
                ..
            } => {
                has_usage = true;
                total_input_tokens += input_tokens;
                total_output_tokens += output_tokens;
                if let Some(cost) = total_cost_usd {
                    total_cost += cost;
                }
            }
            _ => {}
        }
    }

    // Compute duration
    let duration_secs = match (&first_ts, &last_ts) {
        (Some(first), Some(last)) => {
            let first_dt = chrono::DateTime::parse_from_rfc3339(first).ok();
            let last_dt = chrono::DateTime::parse_from_rfc3339(last).ok();
            match (first_dt, last_dt) {
                (Some(f), Some(l)) => {
                    Some(l.signed_duration_since(f).num_milliseconds() as f64 / 1000.0)
                }
                _ => None,
            }
        }
        _ => None,
    };

    let total_tool_calls: u32 = tool_calls.values().sum();

    Ok(SessionSummary {
        session_id: session_id.to_string(),
        name,
        provider,
        model,
        status,
        duration_secs,
        turns,
        tool_calls,
        total_tool_calls,
        files_modified,
        result: last_assistant_msg.map(|s| s.chars().take(500).collect()),
        error,
        event_count,
        input_tokens: if has_usage {
            Some(total_input_tokens)
        } else {
            None
        },
        output_tokens: if has_usage {
            Some(total_output_tokens)
        } else {
            None
        },
        total_cost_usd: if has_usage && total_cost > 0.0 {
            Some(total_cost)
        } else {
            None
        },
    })
}

/// Format duration in human-readable form.
fn format_duration(secs: f64) -> String {
    if secs < 60.0 {
        format!("{:.0}s", secs)
    } else if secs < 3600.0 {
        let mins = (secs / 60.0).floor();
        let remaining = secs - (mins * 60.0);
        format!("{:.0}m {:.0}s", mins, remaining)
    } else {
        let hours = (secs / 3600.0).floor();
        let remaining = secs - (hours * 3600.0);
        let mins = (remaining / 60.0).floor();
        format!("{:.0}h {:.0}m", hours, mins)
    }
}

/// Collect summaries for the given sessions, returning structured data.
pub fn summarize_sessions(params: &SummaryParams) -> Result<Vec<SessionSummary>> {
    let mut session_ids = params.session_ids.clone();

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

    let mut summaries = Vec::new();
    for id in &session_ids {
        if let Ok(s) = summarize_session(id, params.root.as_deref()) {
            summaries.push(s);
        }
    }

    Ok(summaries)
}

/// Run the summary command.
pub fn run_summary(params: SummaryParams) -> Result<()> {
    let summaries = summarize_sessions(&params)?;

    if params.json {
        if summaries.len() == 1 {
            println!("{}", serde_json::to_string_pretty(&summaries[0])?);
        } else {
            println!("{}", serde_json::to_string_pretty(&summaries)?);
        }
    } else {
        for s in &summaries {
            let id_short = &s.session_id[..s.session_id.len().min(8)];
            let name_display = s.name.as_deref().unwrap_or("");
            let duration_display = s
                .duration_secs
                .map(format_duration)
                .unwrap_or_else(|| "?".to_string());

            let status_colored = match s.status.as_str() {
                "completed" => format!("\x1b[32m{}\x1b[0m", s.status),
                "failed" => format!("\x1b[31m{}\x1b[0m", s.status),
                "running" => format!("\x1b[33m{}\x1b[0m", s.status),
                _ => s.status.clone(),
            };

            println!(
                "Session: {} {} ({}/{}) \u{2014} {} in {}",
                id_short, name_display, s.provider, s.model, status_colored, duration_display
            );

            if !s.files_modified.is_empty() {
                println!("Files modified: {}", s.files_modified.join(", "));
            }

            if !s.tool_calls.is_empty() {
                let tools: Vec<String> = s
                    .tool_calls
                    .iter()
                    .map(|(name, count)| format!("{} ({})", name, count))
                    .collect();
                println!("Tools used: {}", tools.join(", "));
            }

            if params.stats {
                println!(
                    "Turns: {}, Events: {}, Total tool calls: {}",
                    s.turns, s.event_count, s.total_tool_calls
                );
                if let (Some(input), Some(output)) = (s.input_tokens, s.output_tokens) {
                    let cost_str = s
                        .total_cost_usd
                        .map(|c| format!(", Cost: ${:.4}", c))
                        .unwrap_or_default();
                    println!("Tokens: {} in / {} out{}", input, output, cost_str);
                }
            } else {
                println!("Turns: {}", s.turns);
            }

            if let Some(ref result) = s.result {
                let preview: String = result.chars().take(200).collect();
                println!("Result: {}", preview);
            }

            if let Some(ref err) = s.error {
                println!("\x1b[31mError: {}\x1b[0m", err);
            }

            println!();
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "summary_tests.rs"]
mod tests;
