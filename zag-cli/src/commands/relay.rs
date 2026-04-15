//! Relay command: manages a long-lived interactive streaming session.
//!
//! This is a hidden command spawned by `zag spawn --interactive`. It reads
//! user messages from a FIFO (named pipe) and forwards them to the agent's
//! streaming session, while logging all events to the session log.

use anyhow::{Result, bail};
use log::debug;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use zag_agent::factory::AgentFactory;
use zag_agent::output::{ContentBlock, Event};
use zag_agent::session_log::{
    LogCompleteness, LogEventKind, LogSourceKind, SessionLogMetadata, SessionLogWriter, ToolKind,
};

pub(crate) struct RelayParams {
    pub session: String,
    pub provider: String,
    pub model: Option<String>,
    pub root: Option<String>,
    pub auto_approve: bool,
    pub system_prompt: Option<String>,
    pub add_dirs: Vec<String>,
    pub prompt: Option<String>,
}

/// Record a streaming event to the session log.
fn record_event(writer: &SessionLogWriter, event: &Event) -> Result<()> {
    match event {
        Event::Init {
            model,
            working_directory,
            metadata,
            ..
        } => {
            writer.emit(
                LogSourceKind::Wrapper,
                LogEventKind::SessionStarted {
                    command: "interactive".to_string(),
                    model: Some(model.clone()),
                    cwd: working_directory.clone(),
                    resumed: false,
                    backfilled: false,
                },
            )?;
            // Try to extract provider session ID from metadata
            if let Some(session_id) = metadata
                .get("session_id")
                .and_then(|v| v.as_str())
                .map(str::to_string)
            {
                writer.set_provider_session_id(Some(session_id))?;
            }
        }
        Event::UserMessage { content } => {
            for block in content {
                if let ContentBlock::Text { text } = block {
                    writer.emit(
                        LogSourceKind::Wrapper,
                        LogEventKind::UserMessage {
                            role: "user".to_string(),
                            content: text.clone(),
                            message_id: None,
                        },
                    )?;
                }
            }
        }
        Event::AssistantMessage { content, .. } => {
            for block in content {
                match block {
                    ContentBlock::Text { text } => {
                        writer.emit(
                            LogSourceKind::Wrapper,
                            LogEventKind::AssistantMessage {
                                content: text.clone(),
                                message_id: None,
                            },
                        )?;
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        writer.emit(
                            LogSourceKind::Wrapper,
                            LogEventKind::ToolCall {
                                tool_kind: Some(ToolKind::infer(name)),
                                tool_name: name.clone(),
                                tool_id: Some(id.clone()),
                                input: Some(input.clone()),
                            },
                        )?;
                    }
                }
            }
        }
        Event::ToolExecution {
            tool_name,
            tool_id,
            result,
            ..
        } => {
            writer.emit(
                LogSourceKind::Wrapper,
                LogEventKind::ToolResult {
                    tool_kind: Some(ToolKind::infer(tool_name)),
                    tool_name: Some(tool_name.clone()),
                    tool_id: Some(tool_id.clone()),
                    success: Some(result.success),
                    output: result.output.clone(),
                    error: result.error.clone(),
                    data: result.data.clone(),
                },
            )?;
        }
        Event::PermissionRequest {
            tool_name,
            description,
            granted,
        } => {
            writer.emit(
                LogSourceKind::Wrapper,
                LogEventKind::Permission {
                    tool_name: tool_name.clone(),
                    description: description.clone(),
                    granted: *granted,
                },
            )?;
        }
        Event::Error { message, details } => {
            writer.emit(
                LogSourceKind::Wrapper,
                LogEventKind::ProviderStatus {
                    message: message.clone(),
                    data: details.clone(),
                },
            )?;
        }
        Event::Result {
            success,
            message,
            duration_ms,
            num_turns,
        } => {
            writer.emit(
                LogSourceKind::Wrapper,
                LogEventKind::ProviderStatus {
                    message: message
                        .clone()
                        .unwrap_or_else(|| "Result emitted".to_string()),
                    data: Some(serde_json::json!({
                        "success": success,
                        "duration_ms": duration_ms,
                        "num_turns": num_turns,
                    })),
                },
            )?;
        }
        Event::TurnComplete {
            stop_reason,
            turn_index,
            usage,
        } => {
            writer.emit(
                LogSourceKind::Wrapper,
                LogEventKind::ProviderStatus {
                    message: format!("Turn {turn_index} complete"),
                    data: Some(serde_json::json!({
                        "stop_reason": stop_reason,
                        "turn_index": turn_index,
                        "usage": usage,
                    })),
                },
            )?;
        }
    }
    Ok(())
}

pub(crate) async fn run_relay(params: RelayParams) -> Result<()> {
    if params.provider != "claude" {
        bail!(
            "Interactive sessions currently require the Claude provider (got '{}')",
            params.provider
        );
    }

    let fifo_path = zag_orch::spawn::fifo_path(&params.session);
    if !fifo_path.exists() {
        bail!("FIFO not found at {}", fifo_path.display());
    }

    debug!(
        "Starting relay: session={}, provider={}, fifo={}",
        params.session,
        params.provider,
        fifo_path.display()
    );

    // Set up session log writer
    let logs_dir = zag_orch::util::logs_dir(params.root.as_deref());
    let writer = SessionLogWriter::create(
        &logs_dir,
        SessionLogMetadata {
            provider: params.provider.clone(),
            wrapper_session_id: params.session.clone(),
            provider_session_id: None,
            workspace_path: params.root.clone(),
            command: "interactive".to_string(),
            model: params.model.clone(),
            resumed: false,
            backfilled: false,
        },
    )?;
    writer.set_completeness(LogCompleteness::Full)?;

    let writer = Arc::new(writer);

    // Create agent and start streaming session
    let mut agent = AgentFactory::create(
        &params.provider,
        params.system_prompt,
        params.model,
        params.root,
        params.auto_approve,
        params.add_dirs,
    )?;

    let claude_agent = agent
        .as_any_mut()
        .downcast_mut::<crate::claude::Claude>()
        .expect("Provider is claude but downcast failed");

    let mut session = claude_agent.execute_streaming(params.prompt.as_deref())?;

    // Open FIFO with O_RDWR to prevent EOF when writers disconnect
    #[cfg(unix)]
    let fifo_file = {
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&fifo_path)?
    };
    #[cfg(not(unix))]
    let fifo_file = std::fs::File::open(&fifo_path)?;

    let fifo_async = tokio::fs::File::from_std(fifo_file);
    let fifo_reader = BufReader::new(fifo_async);
    let mut fifo_lines = fifo_reader.lines();

    // FIFO input task: read messages and forward to agent
    let writer_for_output = Arc::clone(&writer);

    // Use tokio::select to concurrently read from FIFO and agent
    loop {
        tokio::select! {
            // Read from FIFO
            line_result = fifo_lines.next_line() => {
                match line_result {
                    Ok(Some(line)) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        // Parse as NDJSON to extract content, or send raw
                        let content = if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
                            json.get("content")
                                .and_then(|v| v.as_str())
                                .unwrap_or(trimmed)
                                .to_string()
                        } else {
                            trimmed.to_string()
                        };
                        debug!("Relay: sending user message ({} bytes)", content.len());
                        if let Err(e) = session.send_user_message(&content).await {
                            log::error!("Failed to send message to agent: {e}");
                            break;
                        }
                    }
                    Ok(None) => {
                        // FIFO EOF — with O_RDWR this shouldn't happen, but handle it
                        debug!("Relay: FIFO EOF");
                        break;
                    }
                    Err(e) => {
                        // EAGAIN/EWOULDBLOCK is expected for non-blocking reads
                        if e.kind() == std::io::ErrorKind::WouldBlock {
                            // No data available, yield to let output task run
                            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                            continue;
                        }
                        log::error!("Relay: FIFO read error: {e}");
                        break;
                    }
                }
            }
            // Read from agent
            event_result = session.next_event() => {
                match event_result {
                    Ok(Some(event)) => {
                        if let Err(e) = record_event(&writer_for_output, &event) {
                            log::warn!("Failed to record event: {e}");
                        }
                    }
                    Ok(None) => {
                        // Agent process exited
                        debug!("Relay: agent stream ended");
                        break;
                    }
                    Err(e) => {
                        log::error!("Relay: agent event error: {e}");
                        break;
                    }
                }
            }
        }
    }

    // Clean up
    writer.emit(
        LogSourceKind::Wrapper,
        LogEventKind::SessionEnded {
            success: true,
            error: None,
        },
    )?;

    // Remove the FIFO
    let _ = std::fs::remove_file(&fifo_path);

    // Update process store
    let mut proc_store = zag_agent::process_store::ProcessStore::load().unwrap_or_default();
    if let Some(entry) = proc_store
        .processes
        .iter_mut()
        .find(|e| e.session_id.as_deref() == Some(&params.session))
    {
        entry.status = "exited".to_string();
        entry.exit_code = Some(0);
        entry.exited_at = Some(chrono::Utc::now().to_rfc3339());
    }
    if let Err(e) = proc_store.save() {
        log::warn!("Failed to update process store: {e}");
    }

    // Try to wait for the agent process to finish
    let _ = session.wait().await;

    debug!("Relay: session {} ended", params.session);
    Ok(())
}

#[cfg(test)]
#[path = "relay_tests.rs"]
mod tests;
