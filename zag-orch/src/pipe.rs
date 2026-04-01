//! Pipe command: chain session results into a new session.
//!
//! Collects results from one or more completed sessions and feeds them
//! as context into a new agent session with a user-provided prompt.

use crate::collect::extract_last_assistant_message;
use anyhow::{Result, bail};
use log::debug;
use zag::session::SessionStore;

/// Parameters for the pipe command.
pub struct PipeParams {
    pub session_ids: Vec<String>,
    pub tag: Option<String>,
    pub prompt: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub root: Option<String>,
    pub auto_approve: bool,
    pub system_prompt: Option<String>,
    pub add_dirs: Vec<String>,
    pub size: Option<String>,
    pub max_turns: Option<u32>,
    pub output: Option<String>,
    pub json: bool,
    pub quiet: bool,
}

/// Resolve session IDs from explicit IDs and/or tag.
fn resolve_pipe_sessions(
    session_ids: &[String],
    tag: Option<&str>,
    root: Option<&str>,
) -> Result<Vec<String>> {
    let mut ids = session_ids.to_vec();

    if let Some(t) = tag {
        let store = SessionStore::load(root)?;
        let tagged = store.find_by_tag(t);
        if tagged.is_empty() && ids.is_empty() {
            bail!("No sessions found with tag '{}'", t);
        }
        for entry in tagged {
            if !ids.contains(&entry.session_id) {
                ids.push(entry.session_id.clone());
            }
        }
    }

    if ids.is_empty() {
        bail!("No sessions specified. Provide session IDs or --tag.");
    }

    Ok(ids)
}

/// Build a context prefix from session results.
fn build_context(session_ids: &[String], root: Option<&str>) -> Result<String> {
    let mut parts = Vec::new();

    for (i, id) in session_ids.iter().enumerate() {
        let result = extract_last_assistant_message(id, root);
        match result {
            Some(text) => {
                if session_ids.len() == 1 {
                    parts.push(format!(
                        "<session-result session=\"{}\">\n{}\n</session-result>",
                        &id[..id.len().min(8)],
                        text
                    ));
                } else {
                    parts.push(format!(
                        "<session-result index=\"{}\" session=\"{}\">\n{}\n</session-result>",
                        i + 1,
                        &id[..id.len().min(8)],
                        text
                    ));
                }
            }
            None => {
                log::warn!("No result found for session {}", id);
            }
        }
    }

    if parts.is_empty() {
        bail!("No results available from the specified sessions");
    }

    Ok(parts.join("\n\n"))
}

/// Run the pipe command.
pub async fn run_pipe(params: PipeParams) -> Result<()> {
    let session_ids = resolve_pipe_sessions(
        &params.session_ids,
        params.tag.as_deref(),
        params.root.as_deref(),
    )?;

    debug!(
        "Pipe: collecting results from {} session(s)",
        session_ids.len()
    );

    let context = build_context(&session_ids, params.root.as_deref())?;
    let full_prompt = format!(
        "Here are results from previous agent sessions:\n\n{}\n\n{}",
        context, params.prompt
    );

    debug!(
        "Pipe: running exec with combined prompt ({} bytes)",
        full_prompt.len()
    );

    // Resolve provider
    let provider =
        zag::config::resolve_provider(params.provider.as_deref(), params.root.as_deref())?;

    let mut builder = zag::builder::AgentBuilder::new().provider(&provider);

    if let Some(ref model) = params.model {
        builder = builder.model(model);
    }
    if let Some(ref root) = params.root {
        builder = builder.root(root);
    }
    if params.auto_approve {
        builder = builder.auto_approve(true);
    }
    if let Some(ref sp) = params.system_prompt {
        builder = builder.system_prompt(sp);
    }
    for dir in &params.add_dirs {
        builder = builder.add_dir(dir);
    }
    if let Some(ref size) = params.size {
        builder = builder.size(size);
    }
    if let Some(turns) = params.max_turns {
        builder = builder.max_turns(turns);
    }
    if params.quiet {
        builder = builder.quiet(true);
    }

    let output = builder.exec(&full_prompt).await?;

    // Output the result
    let format = params
        .output
        .as_deref()
        .unwrap_or(if params.json { "json" } else { "text" });

    match format {
        "json" => println!("{}", serde_json::to_string(&output)?),
        "json-pretty" => println!("{}", serde_json::to_string_pretty(&output)?),
        _ => {
            if let Some(text) = output.final_result() {
                println!("{}", text);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "pipe_tests.rs"]
mod tests;
