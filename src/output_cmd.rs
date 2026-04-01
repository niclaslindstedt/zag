//! Output command: extract the final result text from a session.
//!
//! Prints just the last assistant message text, making it easy to use
//! session results in shell pipelines without parsing JSON.

use crate::collect::extract_last_assistant_message;
use anyhow::{Result, bail};
use zag::session::SessionStore;

/// Parameters for the output command.
pub struct OutputParams {
    pub session_id: Option<String>,
    pub latest: bool,
    pub output_name: Option<String>,
    pub tag: Option<String>,
    pub json: bool,
    pub root: Option<String>,
}

/// Resolve session IDs from the various targeting flags.
fn resolve_session_ids(params: &OutputParams) -> Result<Vec<String>> {
    let store = SessionStore::load(params.root.as_deref()).unwrap_or_default();

    if let Some(ref id) = params.session_id {
        return Ok(vec![id.clone()]);
    }

    if let Some(ref name) = params.output_name {
        if let Some(entry) = store.find_by_name(name) {
            return Ok(vec![entry.session_id.clone()]);
        }
        bail!("No session found with name '{}'", name);
    }

    if let Some(ref tag) = params.tag {
        let tagged = store.find_by_tag(tag);
        if tagged.is_empty() {
            bail!("No sessions found with tag '{}'", tag);
        }
        return Ok(tagged.iter().map(|e| e.session_id.clone()).collect());
    }

    if params.latest {
        if let Some(entry) = store.latest() {
            return Ok(vec![entry.session_id.clone()]);
        }
        bail!("No sessions found");
    }

    // Default: latest session
    if let Some(entry) = store.latest() {
        return Ok(vec![entry.session_id.clone()]);
    }
    bail!("No sessions found. Use a session ID, --latest, --name, or --tag.");
}

/// Run the output command.
pub fn run_output(params: OutputParams) -> Result<()> {
    let session_ids = resolve_session_ids(&params)?;

    if params.json {
        let mut results = Vec::new();
        for id in &session_ids {
            let text = extract_last_assistant_message(id, params.root.as_deref());
            results.push(serde_json::json!({
                "session_id": id,
                "result": text,
            }));
        }
        if results.len() == 1 {
            println!("{}", serde_json::to_string(&results[0])?);
        } else {
            println!("{}", serde_json::to_string(&results)?);
        }
    } else {
        for (i, id) in session_ids.iter().enumerate() {
            let text = extract_last_assistant_message(id, params.root.as_deref());
            if let Some(text) = text {
                if session_ids.len() > 1 && i > 0 {
                    println!();
                }
                print!("{}", text);
            }
        }
        // Ensure trailing newline for single results
        if session_ids.len() == 1 {
            println!();
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "output_cmd_tests.rs"]
mod tests;
