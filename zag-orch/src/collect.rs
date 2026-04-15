//! Collect command: gather results from multiple sessions.

use crate::listen;
use anyhow::{Result, bail};
use std::io::{BufRead, BufReader};
use zag_agent::session::SessionStore;
use zag_agent::session_log::{AgentLogEvent, LogEventKind};

/// A collected result from a single session.
#[derive(Debug, serde::Serialize)]
pub struct CollectedResult {
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub provider: String,
    pub model: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Parameters for the collect command.
pub struct CollectParams {
    pub session_ids: Vec<String>,
    pub tag: Option<String>,
    pub json: bool,
    pub root: Option<String>,
}

/// Extract just the last assistant message text from a session log.
pub fn extract_last_assistant_message(session_id: &str, root: Option<&str>) -> Option<String> {
    let (_, text, _) = extract_result(session_id, root);
    text
}

/// Extract the last assistant message and session status from a log file.
fn extract_result(
    session_id: &str,
    root: Option<&str>,
) -> (String, Option<String>, Option<String>) {
    let log_path = listen::resolve_session_log(Some(session_id), false, false, root);
    let Ok(path) = log_path else {
        return ("unknown".to_string(), None, None);
    };

    let file = match std::fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => return ("unknown".to_string(), None, None),
    };

    let reader = BufReader::new(file);
    let mut last_assistant_msg: Option<String> = None;
    let mut status = "unknown".to_string();
    let mut error: Option<String> = None;

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
            match &event.kind {
                LogEventKind::AssistantMessage { content, .. } => {
                    last_assistant_msg = Some(content.clone());
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
                _ => {}
            }
        }
    }

    if status == "unknown" {
        status = "running".to_string();
    }

    (status, last_assistant_msg, error)
}

/// Collect results from multiple sessions, returning structured data.
pub fn collect_results(params: &CollectParams) -> Result<Vec<CollectedResult>> {
    let store = SessionStore::load(params.root.as_deref())?;
    let mut session_ids: Vec<String> = params.session_ids.clone();

    if let Some(ref tag) = params.tag {
        let tagged = store.find_by_tag(tag);
        if tagged.is_empty() && session_ids.is_empty() {
            bail!("No sessions found with tag '{tag}'");
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
        let entry = store.find_by_any_id(id);
        let (provider, model, name) = match entry {
            Some(e) => (e.provider.clone(), e.model.clone(), e.name.clone()),
            None => (String::new(), String::new(), None),
        };

        let (status, result_text, error) = extract_result(id, params.root.as_deref());

        results.push(CollectedResult {
            session_id: id.clone(),
            name,
            provider,
            model,
            status,
            result_text,
            error,
        });
    }

    Ok(results)
}

/// Run the collect command (print output wrapper).
pub fn run_collect(params: CollectParams) -> Result<()> {
    let results = collect_results(&params)?;

    if params.json {
        println!("{}", serde_json::to_string(&results)?);
    } else {
        for r in &results {
            let status_colored = match r.status.as_str() {
                "completed" => format!("\x1b[32m{}\x1b[0m", r.status),
                "failed" => format!("\x1b[31m{}\x1b[0m", r.status),
                "running" => format!("\x1b[33m{}\x1b[0m", r.status),
                _ => r.status.clone(),
            };
            let name_display = r.name.as_deref().unwrap_or("-");
            println!(
                "{} [{}] {} ({})",
                r.session_id, status_colored, name_display, r.provider
            );
            if let Some(ref text) = r.result_text {
                let preview: String = text.chars().take(200).collect();
                println!("  {preview}");
            }
            if let Some(ref err) = r.error {
                println!("  \x1b[31merror: {err}\x1b[0m");
            }
            println!();
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "collect_tests.rs"]
mod tests;
