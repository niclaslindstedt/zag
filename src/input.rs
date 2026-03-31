#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;

use anyhow::{Result, bail};
use log::debug;

use crate::factory::AgentFactory;
use crate::listen;
use crate::resume;

pub(crate) struct InputParams {
    pub session: Option<String>,
    pub message: Option<String>,
    pub latest: bool,
    pub active: bool,
    pub ps: Option<String>,
    pub input_name: Option<String>,
    pub input_tag: Option<String>,
    pub broadcast: bool,
    pub global: bool,
    pub stream: bool,
    pub output: Option<String>,
    pub root: Option<String>,
    pub quiet: bool,
    pub raw: bool,
}

struct SenderInfo {
    session_id: String,
    name: Option<String>,
    provider: Option<String>,
    model: Option<String>,
}

impl SenderInfo {
    /// Read sender identity from ZAG_* env vars. Returns None if not inside a session.
    fn from_env() -> Option<Self> {
        let session_id = std::env::var("ZAG_SESSION_ID").ok()?;
        Some(Self {
            session_id,
            name: std::env::var("ZAG_SESSION_NAME").ok(),
            provider: std::env::var("ZAG_PROVIDER").ok(),
            model: std::env::var("ZAG_MODEL").ok(),
        })
    }
}

fn wrap_agent_message(message: &str, sender: &SenderInfo) -> String {
    let provider = sender.provider.as_deref().unwrap_or("unknown");
    let model = sender.model.as_deref().unwrap_or("unknown");
    let name_attr = sender
        .name
        .as_ref()
        .map(|n| format!(" name=\"{}\"", n))
        .unwrap_or_default();
    let reply_target = if let Some(ref name) = sender.name {
        format!("zag input --name {} \"your reply here\"", name)
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

/// If inside a zag session and raw mode is not set, wrap the message with sender metadata.
fn maybe_wrap_message(message: &str, raw: bool) -> String {
    if raw {
        return message.to_string();
    }
    match SenderInfo::from_env() {
        Some(sender) => wrap_agent_message(message, &sender),
        None => message.to_string(),
    }
}

/// Resolve one or more session IDs for the input command from the various targeting flags.
#[allow(clippy::too_many_arguments)]
fn resolve_input_session_ids(
    session: Option<&str>,
    latest: bool,
    active: bool,
    ps: Option<&str>,
    input_name: Option<&str>,
    input_tag: Option<&str>,
    broadcast: bool,
    global: bool,
    root: Option<&str>,
) -> Result<Vec<String>> {
    // --name: resolve by session name
    if let Some(name) = input_name {
        let store = if global {
            zag::session::SessionStore::load_all().unwrap_or_default()
        } else {
            zag::session::SessionStore::load(root).unwrap_or_default()
        };
        if let Some(entry) = store.find_by_name(name) {
            return Ok(vec![entry.session_id.clone()]);
        }
        bail!("No session found with name '{}'", name);
    }

    // --tag: resolve by tag
    if let Some(tag) = input_tag {
        let store = if global {
            zag::session::SessionStore::load_all().unwrap_or_default()
        } else {
            zag::session::SessionStore::load(root).unwrap_or_default()
        };
        let matches = store.find_by_tag(tag);
        if matches.is_empty() {
            bail!("No sessions found with tag '{}'", tag);
        }
        if broadcast {
            return Ok(matches.iter().map(|e| e.session_id.clone()).collect());
        }
        if matches.len() > 1 {
            let mut msg = format!(
                "Multiple sessions found with tag '{}'. Use --broadcast to send to all, or be more specific:\n",
                tag
            );
            for m in &matches {
                msg.push_str(&format!(
                    "  {} ({}{})\n",
                    m.session_id,
                    m.provider,
                    m.name
                        .as_ref()
                        .map(|n| format!(", name={}", n))
                        .unwrap_or_default()
                ));
            }
            bail!("{}", msg.trim_end());
        }
        return Ok(vec![matches[0].session_id.clone()]);
    }

    // Fall back to the single-session resolver
    resolve_input_session_id(session, latest, active, ps, global, root).map(|id| vec![id])
}

/// Resolve the session ID for the input command from the various targeting flags.
fn resolve_input_session_id(
    session: Option<&str>,
    latest: bool,
    active: bool,
    ps: Option<&str>,
    global: bool,
    root: Option<&str>,
) -> Result<String> {
    // --ps resolves via process store
    if let Some(ps_value) = ps {
        return listen::resolve_session_from_ps(ps_value);
    }

    // Direct session ID
    if let Some(id) = session {
        return Ok(id.to_string());
    }

    // --latest or --active: resolve via log path and extract session ID from filename
    if latest || active {
        let log_path = listen::resolve_session_log(None, latest, active, root)?;
        if let Some(stem) = log_path.file_stem().and_then(|s| s.to_str()) {
            return Ok(stem.to_string());
        }
        bail!(
            "Could not extract session ID from log path: {}",
            log_path.display()
        );
    }

    // Auto-resolve: no selector given — find the most recent session
    if global {
        // Search across all projects via global index
        let global_dir = crate::config::Config::global_base_dir();
        if let Ok(index) = zag::session_log::load_global_index(&global_dir) {
            if let Some(entry) = index
                .sessions
                .iter()
                .max_by(|a, b| a.started_at.cmp(&b.started_at))
            {
                return Ok(entry.session_id.clone());
            }
        }
        bail!(
            "No sessions found globally. Use --session, --latest, --active, or --ps to specify one."
        );
    } else {
        // Search current project's session store
        let store = zag::session::SessionStore::load(root).unwrap_or_default();
        if let Some(entry) = store.latest() {
            return Ok(entry.session_id.clone());
        }
        bail!(
            "No sessions found. Use --session, --latest, --active, --ps, or --global to specify one."
        );
    }
}

/// Send a message to multiple sessions (broadcast mode).
async fn run_input_broadcast(
    session_ids: &[String],
    message: Option<String>,
    raw: bool,
    quiet: bool,
    root: Option<&str>,
) -> Result<()> {
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

    for resolved_id in session_ids {
        let target = match resume::resolve_resume_target(resolved_id, root) {
            Some(t) => t,
            None => {
                log::warn!("No session found for '{}', skipping", resolved_id);
                failed += 1;
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

        let agent =
            AgentFactory::create(provider, None, model, root.map(String::from), false, vec![])?;
        match agent
            .run_resume_with_prompt(provider_session_id, &msg)
            .await
        {
            Ok(_) => sent += 1,
            Err(e) => {
                log::warn!("Failed to send to session {}: {}", resolved_id, e);
                failed += 1;
            }
        }
    }

    if !quiet {
        eprintln!(
            "> Sent to {} session{} ({} failed)",
            sent,
            if sent == 1 { "" } else { "s" },
            failed
        );
    }

    Ok(())
}

pub(crate) async fn run_input(params: InputParams) -> Result<()> {
    let InputParams {
        session,
        message,
        latest,
        active,
        ps,
        input_name,
        input_tag,
        broadcast,
        global,
        stream,
        output,
        root,
        quiet,
        raw,
    } = params;

    // Resolve the target session(s)
    let resolved_ids = resolve_input_session_ids(
        session.as_deref(),
        latest,
        active,
        ps.as_deref(),
        input_name.as_deref(),
        input_tag.as_deref(),
        broadcast,
        global,
        root.as_deref(),
    )?;

    // Broadcast mode: send to all resolved sessions
    if resolved_ids.len() > 1 {
        return run_input_broadcast(&resolved_ids, message, raw, quiet, root.as_deref()).await;
    }

    let resolved_id = resolved_ids
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No matching session found"))?;
    debug!("Input command: resolved session ID = {}", resolved_id);

    // Resolve the resume target to get provider, provider_session_id, model
    let target = resume::resolve_resume_target(&resolved_id, root.as_deref())
        .ok_or_else(|| anyhow::anyhow!("No session found for '{}'", resolved_id))?;

    let provider = &target.entry.provider;
    let provider_session_id = target
        .entry
        .provider_session_id
        .as_deref()
        .unwrap_or(&resolved_id);
    let model = if target.entry.model.is_empty() {
        None
    } else {
        Some(target.entry.model.clone())
    };

    debug!(
        "Input command: provider={}, provider_session_id={}, model={:?}",
        provider, provider_session_id, model
    );

    if !quiet {
        log::info!(
            "Sending to {} session {}",
            crate::capitalize(provider),
            &resolved_id[..resolved_id.len().min(8)]
        );
    }

    if stream {
        // Streaming mode: Claude only
        if provider != "claude" {
            bail!("Streaming input (--stream) is only supported for Claude sessions");
        }

        let mut agent = AgentFactory::create(provider, None, model, root.clone(), false, vec![])?;
        let claude_agent = agent
            .as_any_mut()
            .downcast_mut::<crate::claude::Claude>()
            .expect("Failed to get Claude agent");

        let mut session = claude_agent.execute_streaming_resume(provider_session_id)?;

        // Read lines from stdin and send as user messages, while streaming output events
        let stdin_task = {
            let stdin = tokio::io::stdin();
            let reader = tokio::io::BufReader::new(stdin);
            tokio::spawn(async move {
                use tokio::io::AsyncBufReadExt;
                let mut lines = reader.lines();
                let mut messages = Vec::new();
                while let Ok(Some(line)) = lines.next_line().await {
                    if !line.is_empty() {
                        messages.push(line);
                    }
                }
                messages
            })
        };

        // Send messages and read events concurrently
        let messages = stdin_task.await?;
        for msg in messages {
            let wrapped = maybe_wrap_message(&msg, raw);
            session.send_user_message(&wrapped).await?;
        }
        session.close_input();

        // Read all events from the session
        let output_format = output.as_deref().unwrap_or("text");
        while let Some(event) = session.next_event().await? {
            match output_format {
                "json" | "stream-json" => {
                    println!("{}", serde_json::to_string(&event)?);
                }
                _ => {
                    // Text output: print assistant messages
                    if let zag::output::Event::AssistantMessage { ref content, .. } = event {
                        for block in content {
                            if let zag::output::ContentBlock::Text { text } = block {
                                print!("{}", text);
                            }
                        }
                    }
                }
            }
        }

        session.wait().await?;
    } else {
        // Single message mode: resolve the message
        let msg = if let Some(m) = message {
            m
        } else {
            // Read from stdin
            let mut buf = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
            let trimmed = buf.trim().to_string();
            if trimmed.is_empty() {
                bail!("No message provided. Pass a message argument or pipe to stdin.");
            }
            trimmed
        };

        let msg = maybe_wrap_message(&msg, raw);

        debug!("Input command: sending message ({} bytes)", msg.len());

        let mut agent = AgentFactory::create(provider, None, model, root.clone(), false, vec![])?;

        let output_format = output.as_deref();

        // For Claude with stream-json output, use streaming resume
        if provider == "claude" && output_format == Some("stream-json") {
            let claude_agent = agent
                .as_any_mut()
                .downcast_mut::<crate::claude::Claude>()
                .expect("Failed to get Claude agent");

            let mut session = claude_agent.execute_streaming_resume(provider_session_id)?;
            session.send_user_message(&msg).await?;
            session.close_input();

            while let Some(event) = session.next_event().await? {
                println!("{}", serde_json::to_string(&event)?);
            }

            session.wait().await?;
        } else {
            // Use run_resume_with_prompt for all providers
            match agent
                .run_resume_with_prompt(provider_session_id, &msg)
                .await?
            {
                Some(agent_output) => {
                    let format = output_format.unwrap_or("text");
                    match format {
                        "json" => {
                            println!("{}", serde_json::to_string(&agent_output)?);
                        }
                        "json-pretty" => {
                            println!("{}", serde_json::to_string_pretty(&agent_output)?);
                        }
                        _ => {
                            // Text output: print the final result
                            if let Some(text) = agent_output.final_result() {
                                println!("{}", text);
                            }
                        }
                    }
                }
                None => {
                    if !quiet {
                        eprintln!("Agent produced no output");
                    }
                }
            }
        }
    }

    Ok(())
}
