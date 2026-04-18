//! Pipe command: chain session results into a new session.
//!
//! Collects results from one or more completed sessions and feeds them
//! as context into a new agent session with a user-provided prompt.

use crate::collect::extract_last_assistant_message;
use crate::duration::parse_duration;
use crate::types::SessionMetadata;
use anyhow::{Result, bail};
use log::debug;
use zag_agent::session::SessionStore;

/// Parameters for the pipe command.
pub struct PipeParams {
    pub session_ids: Vec<String>,
    /// Input session filter — include all sessions whose `tags` contain this
    /// value as additional pipe inputs. Distinct from `metadata.tags`, which
    /// tags the *new* session launched by pipe.
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
    /// Session metadata (`name`, `description`, `tags`) applied to the new
    /// session launched by pipe. Mirrors the `--name` / `--description` /
    /// `--tag` flags that `run`, `exec`, and `spawn` already accept.
    pub metadata: SessionMetadata,
    /// Kill the agent if it hasn't completed within this duration. Parsed by
    /// [`crate::duration::parse_duration`] (e.g. `"30s"`, `"5m"`, `"1h"`).
    pub timeout: Option<String>,
    /// Extra environment variables set on the agent subprocess.
    pub env_vars: Vec<(String, String)>,
    /// Files attached to the prompt (text files inlined, others referenced).
    pub files: Vec<String>,
    /// Create a git worktree for the new session. `Some(None)` for a
    /// generated name, `Some(Some(name))` for an explicit one, `None` to
    /// run in place.
    pub worktree: Option<Option<String>>,
    /// Run the new session inside a Docker sandbox. `Some(None)` for a
    /// generated name, `Some(Some(name))` for an explicit one.
    pub sandbox: Option<Option<String>>,
    /// Prepend the last assistant message from this prior session to the
    /// combined prompt. Equivalent to the `--context` flag on run/exec.
    pub context: Option<String>,
    /// MCP server config: JSON string or path to a JSON file (Claude only).
    pub mcp_config: Option<String>,
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
            bail!("No sessions found with tag '{t}'");
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
                log::warn!("No result found for session {id}");
            }
        }
    }

    if parts.is_empty() {
        bail!("No results available from the specified sessions");
    }

    Ok(parts.join("\n\n"))
}

/// Pipe session results into a new agent and return the output.
pub async fn pipe_sessions(params: &PipeParams) -> Result<zag_agent::output::AgentOutput> {
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
    let mut full_prompt = format!(
        "Here are results from previous agent sessions:\n\n{}\n\n{}",
        context, params.prompt
    );

    if let Some(ref ctx_id) = params.context {
        if let Some(ctx_text) = extract_last_assistant_message(ctx_id, params.root.as_deref()) {
            full_prompt = format!(
                "Context from previous session ({ctx_id}):\n\n{ctx_text}\n\n---\n\n{full_prompt}"
            );
        } else {
            log::warn!("No context found for session {ctx_id}");
        }
    }

    debug!(
        "Pipe: running exec with combined prompt ({} bytes)",
        full_prompt.len()
    );

    let provider =
        zag_agent::config::resolve_provider(params.provider.as_deref(), params.root.as_deref())?;

    let mut builder = zag_agent::builder::AgentBuilder::new().provider(&provider);

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

    if let Some(ref name) = params.metadata.name {
        builder = builder.name(name);
    }
    if let Some(ref desc) = params.metadata.description {
        builder = builder.description(desc);
    }
    for tag in &params.metadata.tags {
        builder = builder.tag(tag);
    }
    if let Some(ref timeout_str) = params.timeout {
        builder = builder.timeout(parse_duration(timeout_str)?);
    }
    for (key, value) in &params.env_vars {
        builder = builder.env(key, value);
    }
    for file in &params.files {
        builder = builder.file(file);
    }
    if let Some(ref worktree_opt) = params.worktree {
        builder = builder.worktree(worktree_opt.as_deref());
    }
    if let Some(ref sandbox_opt) = params.sandbox {
        builder = builder.sandbox(sandbox_opt.as_deref());
    }
    if let Some(ref mcp) = params.mcp_config {
        builder = builder.mcp_config(mcp);
    }

    builder.exec(&full_prompt).await
}

/// Run the pipe command.
pub async fn run_pipe(params: PipeParams) -> Result<()> {
    let output = pipe_sessions(&params).await?;

    let format = params
        .output
        .as_deref()
        .unwrap_or(if params.json { "json" } else { "text" });

    match format {
        "json" => println!("{}", serde_json::to_string(&output)?),
        "json-pretty" => println!("{}", serde_json::to_string_pretty(&output)?),
        _ => {
            if let Some(text) = output.final_result() {
                println!("{text}");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "pipe_tests.rs"]
mod tests;
