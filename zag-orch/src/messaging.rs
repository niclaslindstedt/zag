//! Library-level helpers for agent-to-agent messaging — the pieces behind
//! `zag input` and `zag broadcast`.
//!
//! Separating these primitives out of the binary lets library users compose
//! their own input/broadcast flows without shelling out to `zag`. The CLI
//! entry points (`zag-cli/src/commands/{input,broadcast}.rs`) are now thin
//! wrappers around this module.

use anyhow::{Result, bail};
use log::debug;
use zag_agent::factory::AgentFactory;
use zag_agent::output::AgentOutput;
use zag_agent::session::{SessionEntry, SessionStore};

use crate::spawn::fifo_path;

/// Identity of the process that is *sending* a message, read from the `ZAG_*`
/// env vars set by a running `zag` session. Used to wrap a message in an
/// `<agent-message>` envelope so the recipient agent knows who is talking to
/// it and how to reply.
pub struct SenderInfo {
    pub session_id: String,
    pub name: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
}

impl SenderInfo {
    /// Read sender identity from `ZAG_*` env vars. Returns `None` when the
    /// current process is not running inside a zag session (i.e. the env
    /// vars are absent).
    pub fn from_env() -> Option<Self> {
        let session_id = std::env::var("ZAG_SESSION_ID").ok()?;
        Some(Self {
            session_id,
            name: std::env::var("ZAG_SESSION_NAME").ok(),
            provider: std::env::var("ZAG_PROVIDER").ok(),
            model: std::env::var("ZAG_MODEL").ok(),
        })
    }
}

/// Wrap `message` in an `<agent-message>` XML envelope carrying the sender's
/// session identity and a suggested reply command.
pub fn wrap_agent_message(message: &str, sender: &SenderInfo) -> String {
    let provider = sender.provider.as_deref().unwrap_or("unknown");
    let model = sender.model.as_deref().unwrap_or("unknown");
    let name_attr = sender
        .name
        .as_ref()
        .map(|n| format!(" name=\"{n}\""))
        .unwrap_or_default();
    let reply_target = if let Some(ref name) = sender.name {
        format!("zag input --name {name} \"your reply here\"")
    } else {
        format!(
            "zag input --session {} \"your reply here\"",
            sender.session_id
        )
    };
    format!(
        "<agent-message>\n\
         <from session=\"{}\"{} provider=\"{}\" model=\"{}\"/>\n\
         <reply-with>{}</reply-with>\n\
         <body>\n\
         {}\n\
         </body>\n\
         </agent-message>",
        sender.session_id, name_attr, provider, model, reply_target, message
    )
}

/// If the current process is inside a zag session (see [`SenderInfo::from_env`])
/// **and** `raw` is false, wrap the message in an `<agent-message>` envelope.
/// Otherwise return the message unchanged.
pub fn maybe_wrap_message(message: &str, raw: bool) -> String {
    if raw {
        return message.to_string();
    }
    match SenderInfo::from_env() {
        Some(sender) => wrap_agent_message(message, &sender),
        None => message.to_string(),
    }
}

/// Resolve every session ID that a broadcast should target. When `tag` is
/// `Some`, restrict to entries with that tag (exact, case-insensitive); when
/// `None`, every session in the scope is included.
///
/// Set `global = true` to search across all projects (`SessionStore::load_all`);
/// otherwise only the current project store (as selected by `root`) is
/// consulted.
pub fn resolve_broadcast_session_ids(
    tag: Option<&str>,
    global: bool,
    root: Option<&str>,
) -> Result<Vec<String>> {
    let store = if global {
        SessionStore::load_all().unwrap_or_default()
    } else {
        SessionStore::load(root).unwrap_or_default()
    };
    if let Some(t) = tag {
        let matches = store.find_by_tag(t);
        if matches.is_empty() {
            bail!("No sessions found with tag '{t}'");
        }
        Ok(matches.iter().map(|e| e.session_id.clone()).collect())
    } else {
        if store.sessions.is_empty() {
            let scope = if global {
                "across all projects"
            } else {
                "in current project"
            };
            bail!("No sessions found {scope}");
        }
        Ok(store
            .sessions
            .iter()
            .map(|e| e.session_id.clone())
            .collect())
    }
}

/// Outcome of a single per-session send within [`send_broadcast`].
#[derive(Debug, Clone)]
pub struct BroadcastOutcome {
    pub session_id: String,
    /// `Ok(())` on success; `Err(msg)` when the send failed.
    pub result: std::result::Result<(), String>,
}

/// Aggregate result of [`send_broadcast`].
#[derive(Debug, Default, Clone)]
pub struct BroadcastResult {
    pub outcomes: Vec<BroadcastOutcome>,
}

impl BroadcastResult {
    pub fn sent(&self) -> usize {
        self.outcomes.iter().filter(|o| o.result.is_ok()).count()
    }

    pub fn failed(&self) -> usize {
        self.outcomes.iter().filter(|o| o.result.is_err()).count()
    }

    pub fn total(&self) -> usize {
        self.outcomes.len()
    }
}

/// Resolve a single session entry by wrapper-or-provider ID against the local
/// session store. Does **not** attempt provider-native discovery; callers that
/// need to discover un-tracked sessions should do that pass themselves before
/// calling the send helpers here.
pub fn lookup_session_entry(session_id: &str, root: Option<&str>) -> Option<SessionEntry> {
    SessionStore::load(root)
        .unwrap_or_default()
        .find_by_any_id(session_id)
        .cloned()
}

/// Send `message` to every session ID in `session_ids`, reusing each
/// session's provider/model. Already-known sessions are loaded from the
/// [`SessionStore`]; unknown ones contribute a failed outcome (no
/// provider-native discovery is attempted here — see [`lookup_session_entry`]).
///
/// Mirrors the behaviour of `zag broadcast`. Returns a [`BroadcastResult`]
/// describing which sessions accepted the send.
pub async fn send_broadcast(
    session_ids: &[String],
    message: &str,
    root: Option<&str>,
) -> Result<BroadcastResult> {
    let mut result = BroadcastResult::default();
    for session_id in session_ids {
        let Some(entry) = lookup_session_entry(session_id, root) else {
            log::warn!("No session found for '{session_id}', skipping");
            result.outcomes.push(BroadcastOutcome {
                session_id: session_id.clone(),
                result: Err("session not found".to_string()),
            });
            continue;
        };

        let provider_session_id = entry
            .provider_session_id
            .as_deref()
            .unwrap_or(session_id)
            .to_string();
        let model = if entry.model.is_empty() {
            None
        } else {
            Some(entry.model.clone())
        };

        let agent = AgentFactory::create(
            &entry.provider,
            None,
            model,
            root.map(String::from),
            false,
            Vec::new(),
        )?;
        match agent
            .run_resume_with_prompt(&provider_session_id, message)
            .await
        {
            Ok(_) => result.outcomes.push(BroadcastOutcome {
                session_id: session_id.clone(),
                result: Ok(()),
            }),
            Err(e) => {
                log::warn!("Failed to send to session {session_id}: {e}");
                result.outcomes.push(BroadcastOutcome {
                    session_id: session_id.clone(),
                    result: Err(e.to_string()),
                });
            }
        }
    }
    Ok(result)
}

/// Send a single message to a non-interactive session (resume + prompt
/// variant). Returns the final [`AgentOutput`] for logging/printing, or
/// `None` when the agent produced no structured output.
pub async fn send_input_once(
    provider: &str,
    provider_session_id: &str,
    model: Option<String>,
    message: &str,
    root: Option<String>,
) -> Result<Option<AgentOutput>> {
    debug!(
        "send_input_once: provider={provider} session={} bytes={}",
        provider_session_id,
        message.len()
    );
    let agent = AgentFactory::create(provider, None, model, root, false, Vec::new())?;
    agent
        .run_resume_with_prompt(provider_session_id, message)
        .await
}

/// Write a single NDJSON `user_message` to an interactive session's FIFO.
///
/// The FIFO path is resolved via [`crate::spawn::fifo_path`]; the caller is
/// expected to have verified that the FIFO exists. Blocks until a reader is
/// available on the other end (kernel semantics of a named pipe).
pub async fn send_via_fifo(session_id: &str, message: &str) -> Result<()> {
    let fifo = fifo_path(session_id);
    if !fifo.exists() {
        bail!(
            "No FIFO for session {session_id} at {} — is the interactive relay running?",
            fifo.display()
        );
    }
    let ndjson = serde_json::json!({
        "type": "user_message",
        "content": message,
    });
    let line = format!("{}\n", serde_json::to_string(&ndjson)?);

    use tokio::io::AsyncWriteExt;
    let mut fifo_file = tokio::fs::OpenOptions::new()
        .write(true)
        .open(&fifo)
        .await?;
    fifo_file.write_all(line.as_bytes()).await?;
    fifo_file.flush().await?;
    Ok(())
}

#[cfg(test)]
#[path = "messaging_tests.rs"]
mod tests;
