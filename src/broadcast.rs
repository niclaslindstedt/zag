#[cfg(test)]
#[path = "broadcast_tests.rs"]
mod tests;

use anyhow::{Result, bail};
use log::debug;

use crate::factory::AgentFactory;
use crate::input::maybe_wrap_message;
use crate::resume;

pub(crate) struct BroadcastParams {
    pub message: Option<String>,
    pub tag: String,
    pub global: bool,
    pub output: Option<String>,
    pub root: Option<String>,
    pub quiet: bool,
    pub raw: bool,
}

/// Resolve all session IDs matching a tag.
fn resolve_broadcast_session_ids(
    tag: &str,
    global: bool,
    root: Option<&str>,
) -> Result<Vec<String>> {
    let store = if global {
        zag::session::SessionStore::load_all().unwrap_or_default()
    } else {
        zag::session::SessionStore::load(root).unwrap_or_default()
    };
    let matches = store.find_by_tag(tag);
    if matches.is_empty() {
        bail!("No sessions found with tag '{}'", tag);
    }
    Ok(matches.iter().map(|e| e.session_id.clone()).collect())
}

pub(crate) async fn run_broadcast(params: BroadcastParams) -> Result<()> {
    let BroadcastParams {
        message,
        tag,
        global,
        output,
        root,
        quiet,
        raw,
    } = params;

    let session_ids = resolve_broadcast_session_ids(&tag, global, root.as_deref())?;

    debug!(
        "Broadcast: resolved {} session(s) for tag '{}'",
        session_ids.len(),
        tag
    );

    let msg = if let Some(m) = message {
        m
    } else {
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
        let trimmed = buf.trim().to_string();
        if trimmed.is_empty() {
            bail!("No message provided. Pass a message argument or pipe to stdin.");
        }
        trimmed
    };

    let msg = maybe_wrap_message(&msg, raw);
    let mut sent = 0usize;
    let mut failed = 0usize;
    let mut results: Vec<serde_json::Value> = Vec::new();
    let output_json = matches!(output.as_deref(), Some("json") | Some("json-pretty"));

    for resolved_id in &session_ids {
        let target = match resume::resolve_resume_target(resolved_id, root.as_deref()) {
            Some(t) => t,
            None => {
                log::warn!("No session found for '{}', skipping", resolved_id);
                failed += 1;
                if output_json {
                    results.push(serde_json::json!({
                        "session_id": resolved_id,
                        "status": "failed",
                        "error": "session not found"
                    }));
                }
                continue;
            }
        };

        let provider = &target.entry.provider;
        let provider_session_id = target
            .entry
            .provider_session_id
            .as_deref()
            .unwrap_or(resolved_id);

        let model = if target.entry.model.is_empty() {
            None
        } else {
            Some(target.entry.model.clone())
        };

        let agent = AgentFactory::create(
            provider,
            None,
            model,
            root.as_ref().map(|s| s.to_string()),
            false,
            vec![],
        )?;
        match agent
            .run_resume_with_prompt(provider_session_id, &msg)
            .await
        {
            Ok(_) => {
                sent += 1;
                if output_json {
                    results.push(serde_json::json!({
                        "session_id": resolved_id,
                        "status": "sent"
                    }));
                }
            }
            Err(e) => {
                log::warn!("Failed to send to session {}: {}", resolved_id, e);
                failed += 1;
                if output_json {
                    results.push(serde_json::json!({
                        "session_id": resolved_id,
                        "status": "failed",
                        "error": e.to_string()
                    }));
                }
            }
        }
    }

    if output_json {
        let result = serde_json::json!({
            "results": results,
            "summary": {
                "sent": sent,
                "failed": failed,
                "total": session_ids.len()
            }
        });
        if output.as_deref() == Some("json-pretty") {
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!("{}", serde_json::to_string(&result)?);
        }
    } else if !quiet {
        eprintln!(
            "> Sent to {} session{} ({} failed)",
            sent,
            if sent == 1 { "" } else { "s" },
            failed
        );
    }

    Ok(())
}
