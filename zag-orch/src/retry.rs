//! Retry command: re-run a failed session with the same configuration.
//!
//! Reads the original provider, model, prompt, and tags from the session store
//! and session log, then re-spawns via `run_spawn`.

use crate::listen;
use crate::spawn::{SpawnParams, run_spawn};
use crate::status::{SessionStatus, determine_status};
use crate::types::SessionMetadata;
use anyhow::{Result, bail};
use log::debug;
use std::io::{BufRead, BufReader};
use zag::session::SessionStore;
use zag::session_log::{AgentLogEvent, LogEventKind};

/// Parameters for the retry command.
pub struct RetryParams {
    pub session_ids: Vec<String>,
    pub tag: Option<String>,
    pub failed: bool,
    pub model: Option<String>,
    pub json: bool,
    pub root: Option<String>,
}

/// Result of retrying a single session.
#[derive(Debug, serde::Serialize)]
struct RetryResult {
    original_session_id: String,
    new_session_id: Option<String>,
    retried: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Extract the first user message from a session log (the original prompt).
fn extract_prompt(session_id: &str, root: Option<&str>) -> Option<String> {
    let log_path = listen::resolve_session_log(Some(session_id), false, false, root).ok()?;
    let file = std::fs::File::open(&log_path).ok()?;
    let reader = BufReader::new(file);

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
            if let LogEventKind::UserMessage { content, .. } = &event.kind {
                return Some(content.clone());
            }
        }
    }
    None
}

/// Run the retry command.
pub fn run_retry(params: RetryParams) -> Result<()> {
    let store = SessionStore::load(params.root.as_deref()).unwrap_or_default();
    let mut session_ids = params.session_ids.clone();

    if let Some(ref tag) = params.tag {
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
        // Check if session should be retried
        if params.failed {
            match determine_status(id, params.root.as_deref()) {
                Ok(info) => {
                    if info.status != SessionStatus::Failed && info.status != SessionStatus::Dead {
                        debug!("Skipping session {} (status: {})", id, info.status);
                        continue;
                    }
                }
                Err(_) => {
                    results.push(RetryResult {
                        original_session_id: id.clone(),
                        new_session_id: None,
                        retried: false,
                        error: Some("could not determine session status".to_string()),
                    });
                    continue;
                }
            }
        }

        // Look up the original session entry
        let entry = match store.find_by_any_id(id) {
            Some(e) => e,
            None => {
                results.push(RetryResult {
                    original_session_id: id.clone(),
                    new_session_id: None,
                    retried: false,
                    error: Some("session not found".to_string()),
                });
                continue;
            }
        };

        // Extract the original prompt
        let prompt = match extract_prompt(id, params.root.as_deref()) {
            Some(p) => p,
            None => {
                results.push(RetryResult {
                    original_session_id: id.clone(),
                    new_session_id: None,
                    retried: false,
                    error: Some("could not extract original prompt from session log".to_string()),
                });
                continue;
            }
        };

        let model = params.model.clone().or_else(|| {
            if entry.model.is_empty() {
                None
            } else {
                Some(entry.model.clone())
            }
        });

        debug!(
            "Retrying session {}: provider={}, model={:?}, prompt_len={}",
            id,
            entry.provider,
            model,
            prompt.len()
        );

        // Re-spawn with the same config
        let spawn_result = run_spawn(SpawnParams {
            prompt,
            provider: entry.provider.clone(),
            model,
            root: params.root.clone(),
            auto_approve: false,
            system_prompt: None,
            add_dirs: vec![],
            size: None,
            max_turns: None,
            json: params.json,
            metadata: SessionMetadata {
                name: entry.name.clone(),
                description: entry.description.clone(),
                tags: entry.tags.clone(),
            },
            depends_on: entry.dependencies.clone(),
            inject_context: false,
            retried_from: Some(id.clone()),
        });

        match spawn_result {
            Ok(()) => {
                results.push(RetryResult {
                    original_session_id: id.clone(),
                    new_session_id: None, // spawn prints the ID itself
                    retried: true,
                    error: None,
                });
            }
            Err(e) => {
                results.push(RetryResult {
                    original_session_id: id.clone(),
                    new_session_id: None,
                    retried: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    if !params.json {
        for r in &results {
            if r.retried {
                println!(
                    "\x1b[32m\u{2713}\x1b[0m Retried session {}",
                    r.original_session_id
                );
            } else {
                println!(
                    "\x1b[31m\u{2717}\x1b[0m Failed to retry session {}: {}",
                    r.original_session_id,
                    r.error.as_deref().unwrap_or("unknown error")
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "retry_tests.rs"]
mod tests;
