// provider-updated: 2026-04-05
pub mod logs;
/// Claude agent implementation.
///
/// This module provides the Claude agent implementation, including:
/// - Agent trait implementation for executing Claude commands
/// - JSON output models for parsing Claude's verbose output
/// - Conversion to unified AgentOutput format
pub mod models;

use crate::agent::{Agent, ModelSize};

/// Return the Claude projects directory: `~/.claude/projects/`.
pub fn projects_dir() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude/projects"))
}
use crate::output::AgentOutput;
use crate::providers::common::CommonAgentState;
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

pub const DEFAULT_MODEL: &str = "default";

pub const AVAILABLE_MODELS: &[&str] = &["default", "sonnet", "opus", "haiku"];

/// Callback for streaming events. Set via `set_event_handler` to receive
/// unified events as they arrive during non-interactive execution.
pub type EventHandler = Box<dyn Fn(&crate::output::Event, bool) + Send + Sync>;

pub struct Claude {
    pub common: CommonAgentState,
    pub session_id: Option<String>,
    pub input_format: Option<String>,
    pub verbose: bool,
    pub json_schema: Option<String>,
    pub event_handler: Option<EventHandler>,
    pub replay_user_messages: bool,
    pub include_partial_messages: bool,
    pub mcp_config_path: Option<String>,
}

impl Claude {
    pub fn new() -> Self {
        Self {
            common: CommonAgentState::new(DEFAULT_MODEL),
            session_id: None,
            input_format: None,
            verbose: false,
            json_schema: None,
            event_handler: None,
            replay_user_messages: false,
            include_partial_messages: false,
            mcp_config_path: None,
        }
    }

    pub fn set_input_format(&mut self, format: Option<String>) {
        self.input_format = format;
    }

    pub fn set_session_id(&mut self, session_id: String) {
        self.session_id = Some(session_id);
    }

    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }

    pub fn set_json_schema(&mut self, schema: Option<String>) {
        self.json_schema = schema;
    }

    pub fn set_replay_user_messages(&mut self, replay: bool) {
        self.replay_user_messages = replay;
    }

    pub fn set_include_partial_messages(&mut self, include: bool) {
        self.include_partial_messages = include;
    }

    /// Set MCP server config: a JSON string (written to a temp file) or a file path.
    pub fn set_mcp_config(&mut self, config: Option<String>) {
        self.mcp_config_path = config.map(|c| {
            if c.trim_start().starts_with('{') {
                let path =
                    std::env::temp_dir().join(format!("zag-mcp-{}.json", uuid::Uuid::new_v4()));
                if let Err(e) = std::fs::write(&path, &c) {
                    log::warn!("Failed to write MCP config temp file: {}", e);
                    return c;
                }
                path.to_string_lossy().into_owned()
            } else {
                c
            }
        });
    }

    /// Set a callback to receive streaming events during non-interactive execution.
    ///
    /// The callback receives `(event, verbose)` where `verbose` indicates whether
    /// the user requested verbose output.
    pub fn set_event_handler(&mut self, handler: EventHandler) {
        self.event_handler = Some(handler);
    }

    /// Build the argument list for a run/exec invocation.
    fn build_run_args(
        &self,
        interactive: bool,
        prompt: Option<&str>,
        effective_output_format: &Option<String>,
    ) -> Vec<String> {
        let mut args = Vec::new();
        let in_sandbox = self.common.sandbox.is_some();

        if !interactive {
            args.push("--print".to_string());

            match effective_output_format.as_deref() {
                Some("json") | Some("json-pretty") => {
                    args.extend(["--verbose", "--output-format", "json"].map(String::from));
                }
                Some("stream-json") | None => {
                    args.extend(["--verbose", "--output-format", "stream-json"].map(String::from));
                }
                Some("native-json") => {
                    args.extend(["--verbose", "--output-format", "json"].map(String::from));
                }
                Some("text") => {}
                _ => {}
            }
        }

        // Skip --dangerously-skip-permissions in sandbox (permissions are sandbox-default)
        if self.common.skip_permissions && !in_sandbox {
            args.push("--dangerously-skip-permissions".to_string());
        }

        args.extend(["--model".to_string(), self.common.model.clone()]);

        if interactive && let Some(session_id) = &self.session_id {
            args.extend(["--session-id".to_string(), session_id.clone()]);
        }

        for dir in &self.common.add_dirs {
            args.extend(["--add-dir".to_string(), dir.clone()]);
        }

        if !self.common.system_prompt.is_empty() {
            args.extend([
                "--append-system-prompt".to_string(),
                self.common.system_prompt.clone(),
            ]);
        }

        if !interactive && let Some(ref input_fmt) = self.input_format {
            args.extend(["--input-format".to_string(), input_fmt.clone()]);
        }

        if !interactive && self.replay_user_messages {
            args.push("--replay-user-messages".to_string());
        }

        if !interactive && self.include_partial_messages {
            args.push("--include-partial-messages".to_string());
        }

        if let Some(ref schema) = self.json_schema {
            args.extend(["--json-schema".to_string(), schema.clone()]);
        }

        if let Some(turns) = self.common.max_turns {
            args.extend(["--max-turns".to_string(), turns.to_string()]);
        }

        if let Some(ref path) = self.mcp_config_path {
            args.extend(["--mcp-config".to_string(), path.clone()]);
        }

        if let Some(p) = prompt {
            args.push(p.to_string());
        }

        args
    }

    /// Build the argument list for a resume invocation.
    fn build_resume_args(&self, session_id: Option<&str>) -> Vec<String> {
        let mut args = Vec::new();
        let in_sandbox = self.common.sandbox.is_some();

        if let Some(id) = session_id {
            args.extend(["--resume".to_string(), id.to_string()]);
        } else {
            args.push("--continue".to_string());
        }

        if self.common.skip_permissions && !in_sandbox {
            args.push("--dangerously-skip-permissions".to_string());
        }

        args.extend(["--model".to_string(), self.common.model.clone()]);

        for dir in &self.common.add_dirs {
            args.extend(["--add-dir".to_string(), dir.clone()]);
        }

        args
    }

    /// Create a `Command` either directly or wrapped in sandbox.
    fn make_command(&self, agent_args: Vec<String>) -> Command {
        self.common.make_command("claude", agent_args)
    }

    /// Spawn a streaming session with piped stdin/stdout.
    ///
    /// Automatically configures `--input-format stream-json`, `--output-format stream-json`,
    /// and `--replay-user-messages`. Returns a `StreamingSession` for bidirectional
    /// communication with the agent.
    ///
    /// # Mid-turn semantics
    ///
    /// User messages sent via `StreamingSession::send_user_message` while the
    /// assistant is producing a response are **queued** by the Claude CLI: the
    /// current turn runs to completion and the new message is delivered as the
    /// next user turn. The in-flight turn is **not interrupted**. This
    /// corresponds to `streaming_input.semantics == "queue"` in the capability
    /// descriptor.
    pub fn execute_streaming(
        &self,
        prompt: Option<&str>,
    ) -> Result<crate::streaming::StreamingSession> {
        // Build args for non-interactive streaming mode
        let mut args = Vec::new();
        let in_sandbox = self.common.sandbox.is_some();

        args.push("--print".to_string());
        args.extend(["--verbose", "--output-format", "stream-json"].map(String::from));

        if self.common.skip_permissions && !in_sandbox {
            args.push("--dangerously-skip-permissions".to_string());
        }

        args.extend(["--model".to_string(), self.common.model.clone()]);

        for dir in &self.common.add_dirs {
            args.extend(["--add-dir".to_string(), dir.clone()]);
        }

        if !self.common.system_prompt.is_empty() {
            args.extend([
                "--append-system-prompt".to_string(),
                self.common.system_prompt.clone(),
            ]);
        }

        args.extend(["--input-format".to_string(), "stream-json".to_string()]);
        args.push("--replay-user-messages".to_string());

        if self.include_partial_messages {
            args.push("--include-partial-messages".to_string());
        }

        if let Some(ref schema) = self.json_schema {
            args.extend(["--json-schema".to_string(), schema.clone()]);
        }

        if let Some(p) = prompt {
            args.push(p.to_string());
        }

        log::debug!("Claude streaming command: claude {}", args.join(" "));

        let mut cmd = self.make_command(args);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd
            .spawn()
            .context("Failed to execute 'claude' CLI. Is it installed and in PATH?")?;
        crate::streaming::StreamingSession::new(child)
    }

    /// Build argument list for a streaming resume invocation.
    fn build_streaming_resume_args(&self, session_id: &str) -> Vec<String> {
        let mut args = Vec::new();
        let in_sandbox = self.common.sandbox.is_some();

        args.push("--print".to_string());
        args.extend(["--resume".to_string(), session_id.to_string()]);
        args.extend(["--verbose", "--output-format", "stream-json"].map(String::from));

        if self.common.skip_permissions && !in_sandbox {
            args.push("--dangerously-skip-permissions".to_string());
        }

        args.extend(["--model".to_string(), self.common.model.clone()]);

        for dir in &self.common.add_dirs {
            args.extend(["--add-dir".to_string(), dir.clone()]);
        }

        args.extend(["--input-format".to_string(), "stream-json".to_string()]);
        args.push("--replay-user-messages".to_string());

        if self.include_partial_messages {
            args.push("--include-partial-messages".to_string());
        }

        args
    }

    /// Spawn a streaming session that resumes an existing session.
    ///
    /// Combines `--resume` with `--input-format stream-json`, `--output-format stream-json`,
    /// and `--replay-user-messages`. Returns a `StreamingSession` for bidirectional
    /// communication with the resumed session.
    ///
    /// Mid-turn `send_user_message` calls follow the same **queue** semantics
    /// as [`Self::execute_streaming`].
    pub fn execute_streaming_resume(
        &self,
        session_id: &str,
    ) -> Result<crate::streaming::StreamingSession> {
        let args = self.build_streaming_resume_args(session_id);

        log::debug!("Claude streaming resume command: claude {}", args.join(" "));

        let mut cmd = self.make_command(args);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd
            .spawn()
            .context("Failed to execute 'claude' CLI. Is it installed and in PATH?")?;
        crate::streaming::StreamingSession::new(child)
    }

    async fn execute(
        &self,
        interactive: bool,
        prompt: Option<&str>,
    ) -> Result<Option<AgentOutput>> {
        // When capture_output is set (e.g. by auto-selector), use "json" format
        // so stdout is piped and parsed into AgentOutput
        let effective_output_format =
            if self.common.capture_output && self.common.output_format.is_none() {
                Some("json".to_string())
            } else {
                self.common.output_format.clone()
            };

        // Determine if we should capture structured output
        // Default to streaming unified output when no format is specified in print mode
        let capture_json = !interactive
            && effective_output_format
                .as_ref()
                .is_none_or(|f| f == "json" || f == "json-pretty" || f == "stream-json");

        let agent_args = self.build_run_args(interactive, prompt, &effective_output_format);
        log::debug!("Claude command: claude {}", agent_args.join(" "));
        if !self.common.system_prompt.is_empty() {
            log::debug!("Claude system prompt: {}", self.common.system_prompt);
        }
        if let Some(p) = prompt {
            log::debug!("Claude user prompt: {}", p);
        }
        log::debug!(
            "Claude mode: interactive={}, capture_json={}, output_format={:?}",
            interactive,
            capture_json,
            effective_output_format
        );
        let mut cmd = self.make_command(agent_args);

        // Check if we should pass through native JSON without conversion
        let is_native_json = effective_output_format.as_deref() == Some("native-json");

        if interactive {
            // Interactive mode - inherit all stdio
            cmd.stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());

            let status = cmd
                .status()
                .await
                .context("Failed to execute 'claude' CLI. Is it installed and in PATH?")?;
            if !status.success() {
                return Err(crate::process::ProcessError {
                    exit_code: status.code(),
                    stderr: String::new(),
                    agent_name: "Claude".to_string(),
                }
                .into());
            }
            Ok(None)
        } else if is_native_json {
            // Native JSON mode - pass through Claude's raw JSON output, capture stderr
            cmd.stdin(Stdio::inherit()).stdout(Stdio::inherit());

            crate::process::run_with_captured_stderr(&mut cmd).await?;
            Ok(None)
        } else if capture_json {
            let output_format = effective_output_format.as_deref();
            let is_streaming = output_format == Some("stream-json") || output_format.is_none();

            if is_streaming {
                // For stream-json or default (None), stream output and convert to unified format
                cmd.stdin(Stdio::inherit());
                cmd.stdout(Stdio::piped());

                let mut child = crate::process::spawn_with_captured_stderr(&mut cmd).await?;
                let stdout = child
                    .stdout
                    .take()
                    .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout"))?;

                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();

                // Determine output mode
                let format_as_text = output_format.is_none(); // Default: beautiful text
                let format_as_json = output_format == Some("stream-json"); // Explicit: unified JSON

                // Per-line batch path uses the stateful translator so that
                // TurnComplete events are synthesized alongside Result.
                let mut translator = ClaudeEventTranslator::new();

                // Stream each line, dispatching via event_handler if set
                while let Some(line) = lines.next_line().await? {
                    if format_as_text || format_as_json {
                        match serde_json::from_str::<models::ClaudeEvent>(&line) {
                            Ok(claude_event) => {
                                for unified_event in translator.translate(&claude_event) {
                                    if let Some(ref handler) = self.event_handler {
                                        handler(&unified_event, self.verbose);
                                    }
                                }
                            }
                            Err(e) => {
                                log::debug!(
                                    "Failed to parse streaming Claude event: {}. Line: {}",
                                    e,
                                    crate::truncate_str(&line, 200)
                                );
                            }
                        }
                    }
                }

                // Signal end of streaming to handler
                if let Some(ref handler) = self.event_handler {
                    // Send a Result event to signal completion
                    handler(
                        &crate::output::Event::Result {
                            success: true,
                            message: None,
                            duration_ms: None,
                            num_turns: None,
                        },
                        self.verbose,
                    );
                }

                crate::process::wait_with_stderr(child).await?;

                // Return None to indicate output was streamed directly
                Ok(None)
            } else {
                // For json/json-pretty, capture all output then parse
                cmd.stdin(Stdio::inherit());
                cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

                let output = cmd.output().await?;

                crate::process::handle_output(&output, "Claude")?;

                // Parse JSON output
                let json_str = String::from_utf8(output.stdout)?;
                log::debug!("Parsing Claude JSON output ({} bytes)", json_str.len());
                let claude_output: models::ClaudeOutput =
                    serde_json::from_str(&json_str).map_err(|e| {
                        log::debug!(
                            "Failed to parse Claude JSON output: {}. First 500 chars: {}",
                            e,
                            crate::truncate_str(&json_str, 500)
                        );
                        anyhow::anyhow!("Failed to parse Claude JSON output: {}", e)
                    })?;
                log::debug!("Parsed {} Claude events successfully", claude_output.len());

                // Log any unknown event types for diagnostics.
                if let Ok(raw_events) = serde_json::from_str::<Vec<serde_json::Value>>(&json_str) {
                    let known = ["system", "assistant", "user", "result"];
                    for raw in &raw_events {
                        if let Some(t) = raw.get("type").and_then(|v| v.as_str()) {
                            if !known.contains(&t) {
                                log::debug!(
                                    "Unknown Claude event type: {:?} (first 300 chars: {})",
                                    t,
                                    crate::truncate_str(
                                        &serde_json::to_string(raw).unwrap_or_default(),
                                        300
                                    )
                                );
                            }
                        }
                    }
                }

                // Convert to unified AgentOutput
                let agent_output: AgentOutput =
                    models::claude_output_to_agent_output(claude_output);
                Ok(Some(agent_output))
            }
        } else {
            // Explicit text mode - inherit stdout, capture stderr
            cmd.stdin(Stdio::inherit()).stdout(Stdio::inherit());

            crate::process::run_with_captured_stderr(&mut cmd).await?;
            Ok(None)
        }
    }
}

/// Stateful translator from Claude `stream-json` events to unified
/// [`crate::output::Event`]s.
///
/// Some unified events are synthesized from cross-event state —
/// specifically [`crate::output::Event::TurnComplete`], which carries
/// `stop_reason` and `usage` from the *last* assistant message of a turn
/// and is emitted immediately before the corresponding per-turn
/// [`crate::output::Event::Result`]. This translator owns that state.
///
/// Stateless per-event conversion still goes through
/// [`convert_claude_event_to_unified`]; the translator is a thin stateful
/// wrapper on top.
#[derive(Debug, Default)]
pub(crate) struct ClaudeEventTranslator {
    /// `stop_reason` from the most recent `ClaudeEvent::Assistant` in the
    /// current turn. Consumed when `TurnComplete` is emitted.
    pending_stop_reason: Option<String>,
    /// `usage` from the most recent `ClaudeEvent::Assistant`.
    pending_usage: Option<crate::output::Usage>,
    /// Zero-based turn index within the session. Incremented after each
    /// emitted `TurnComplete`.
    next_turn_index: u32,
    /// Text from the most recent assistant message, used as fallback when
    /// `Result.result` is empty.
    last_assistant_text: Option<String>,
    /// Maps `tool_use_id` → `tool_name` from assistant messages so that
    /// subsequent `ToolExecution` events (which only carry the id) can be
    /// enriched with the correct tool name.
    tool_name_by_id: std::collections::HashMap<String, String>,
}

impl ClaudeEventTranslator {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Translate one Claude event into zero or more unified events.
    ///
    /// A `ClaudeEvent::Result` expands into `[TurnComplete, Result]`; all
    /// other events pass through [`convert_claude_event_to_unified`] and
    /// yield at most one unified event.
    pub(crate) fn translate(&mut self, event: &models::ClaudeEvent) -> Vec<crate::output::Event> {
        use crate::output::{Event as UnifiedEvent, Usage as UnifiedUsage};

        // Observe assistant-side turn state. Every assistant message
        // within the current turn updates the pending stop_reason / usage;
        // the final one wins because it is the message that actually ends
        // the turn (its `stop_reason` will be `end_turn`, `tool_use`,
        // `max_tokens`, or `stop_sequence`).
        if let models::ClaudeEvent::Assistant { message, .. } = event {
            if let Some(reason) = &message.stop_reason {
                self.pending_stop_reason = Some(reason.clone());
            }
            self.pending_usage = Some(UnifiedUsage {
                input_tokens: message.usage.input_tokens,
                output_tokens: message.usage.output_tokens,
                cache_read_tokens: Some(message.usage.cache_read_input_tokens),
                cache_creation_tokens: Some(message.usage.cache_creation_input_tokens),
                web_search_requests: message
                    .usage
                    .server_tool_use
                    .as_ref()
                    .map(|s| s.web_search_requests),
                web_fetch_requests: message
                    .usage
                    .server_tool_use
                    .as_ref()
                    .map(|s| s.web_fetch_requests),
            });

            // Track text for fallback when Result.result is empty.
            let text_parts: Vec<&str> = message
                .content
                .iter()
                .filter_map(|b| match b {
                    models::ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            if !text_parts.is_empty() {
                self.last_assistant_text = Some(text_parts.join("\n"));
            }

            // Track tool_use_id → tool_name so ToolExecution events get the
            // correct name instead of "unknown".
            for block in &message.content {
                if let models::ContentBlock::ToolUse { id, name, .. } = block {
                    self.tool_name_by_id.insert(id.clone(), name.clone());
                }
            }
        }

        let unified = convert_claude_event_to_unified(event);

        match unified {
            Some(UnifiedEvent::Result {
                success,
                message,
                duration_ms,
                num_turns,
            }) if message.as_deref() == Some("") => {
                // Empty result — substitute last assistant text if available.
                let fallback = self.last_assistant_text.take();
                if fallback.is_some() {
                    log::debug!(
                        "Streaming Result.message is empty; using last assistant text as fallback"
                    );
                }
                let result_event = UnifiedEvent::Result {
                    success,
                    message: fallback.or(message),
                    duration_ms,
                    num_turns,
                };
                let turn_complete = UnifiedEvent::TurnComplete {
                    stop_reason: self.pending_stop_reason.take(),
                    turn_index: self.next_turn_index,
                    usage: self.pending_usage.take(),
                };
                self.next_turn_index = self.next_turn_index.saturating_add(1);
                vec![turn_complete, result_event]
            }
            Some(UnifiedEvent::Result { .. }) => {
                let turn_complete = UnifiedEvent::TurnComplete {
                    stop_reason: self.pending_stop_reason.take(),
                    turn_index: self.next_turn_index,
                    usage: self.pending_usage.take(),
                };
                self.next_turn_index = self.next_turn_index.saturating_add(1);
                vec![turn_complete, unified.unwrap()]
            }
            Some(UnifiedEvent::ToolExecution {
                tool_name,
                tool_id,
                input,
                result,
                parent_tool_use_id,
            }) => {
                // Enrich with the real tool name if we tracked it from a
                // prior assistant message.
                let resolved_name = self
                    .tool_name_by_id
                    .get(&tool_id)
                    .cloned()
                    .unwrap_or(tool_name);
                vec![UnifiedEvent::ToolExecution {
                    tool_name: resolved_name,
                    tool_id,
                    input,
                    result,
                    parent_tool_use_id,
                }]
            }
            Some(ev) => vec![ev],
            None => Vec::new(),
        }
    }
}

/// Convert a single Claude event to a unified event format.
/// Returns None if the event doesn't map to a user-visible unified event.
///
/// This is the stateless per-event converter. Callers that need
/// cross-event synthesis (e.g. [`crate::output::Event::TurnComplete`])
/// should use [`ClaudeEventTranslator`] instead.
pub(crate) fn convert_claude_event_to_unified(
    event: &models::ClaudeEvent,
) -> Option<crate::output::Event> {
    use crate::output::{
        ContentBlock as UnifiedContentBlock, Event as UnifiedEvent, ToolResult,
        Usage as UnifiedUsage,
    };
    use models::ClaudeEvent;

    match event {
        ClaudeEvent::System {
            model, tools, cwd, ..
        } => {
            let mut metadata = std::collections::HashMap::new();
            if let Some(cwd_val) = cwd {
                metadata.insert("cwd".to_string(), serde_json::json!(cwd_val));
            }

            Some(UnifiedEvent::Init {
                model: model.clone(),
                tools: tools.clone(),
                working_directory: cwd.clone(),
                metadata,
            })
        }

        ClaudeEvent::Assistant {
            message,
            parent_tool_use_id,
            ..
        } => {
            // Convert content blocks
            let content: Vec<UnifiedContentBlock> = message
                .content
                .iter()
                .filter_map(|block| match block {
                    models::ContentBlock::Text { text } => {
                        Some(UnifiedContentBlock::Text { text: text.clone() })
                    }
                    models::ContentBlock::ToolUse { id, name, input } => {
                        Some(UnifiedContentBlock::ToolUse {
                            id: id.clone(),
                            name: name.clone(),
                            input: input.clone(),
                        })
                    }
                    models::ContentBlock::Thinking { .. } | models::ContentBlock::Other => None,
                })
                .collect();

            // Convert usage
            let usage = Some(UnifiedUsage {
                input_tokens: message.usage.input_tokens,
                output_tokens: message.usage.output_tokens,
                cache_read_tokens: Some(message.usage.cache_read_input_tokens),
                cache_creation_tokens: Some(message.usage.cache_creation_input_tokens),
                web_search_requests: message
                    .usage
                    .server_tool_use
                    .as_ref()
                    .map(|s| s.web_search_requests),
                web_fetch_requests: message
                    .usage
                    .server_tool_use
                    .as_ref()
                    .map(|s| s.web_fetch_requests),
            });

            Some(UnifiedEvent::AssistantMessage {
                content,
                usage,
                parent_tool_use_id: parent_tool_use_id.clone(),
            })
        }

        ClaudeEvent::User {
            message,
            tool_use_result,
            parent_tool_use_id,
            ..
        } => {
            // For streaming, we can't easily look up tool names from previous events
            // So we'll use "unknown" for the tool name in streaming mode
            // Find the first tool_result block (skip text and other blocks)
            let first_tool_result = message.content.iter().find_map(|b| {
                if let models::UserContentBlock::ToolResult {
                    tool_use_id,
                    content,
                    is_error,
                } = b
                {
                    Some((tool_use_id, content, is_error))
                } else {
                    None
                }
            });

            if let Some((tool_use_id, content, is_error)) = first_tool_result {
                let tool_result = ToolResult {
                    success: !is_error,
                    output: if !is_error {
                        Some(content.clone())
                    } else {
                        None
                    },
                    error: if *is_error {
                        Some(content.clone())
                    } else {
                        None
                    },
                    data: tool_use_result.clone(),
                };

                Some(UnifiedEvent::ToolExecution {
                    tool_name: "unknown".to_string(),
                    tool_id: tool_use_id.clone(),
                    input: serde_json::Value::Null,
                    result: tool_result,
                    parent_tool_use_id: parent_tool_use_id.clone(),
                })
            } else {
                // Check for text content (replayed user messages via --replay-user-messages)
                let text_blocks: Vec<UnifiedContentBlock> = message
                    .content
                    .iter()
                    .filter_map(|b| {
                        if let models::UserContentBlock::Text { text } = b {
                            Some(UnifiedContentBlock::Text { text: text.clone() })
                        } else {
                            None
                        }
                    })
                    .collect();

                if !text_blocks.is_empty() {
                    Some(UnifiedEvent::UserMessage {
                        content: text_blocks,
                    })
                } else {
                    None
                }
            }
        }

        ClaudeEvent::Other => {
            log::debug!("Skipping unknown Claude event type during streaming conversion");
            None
        }

        ClaudeEvent::Result {
            is_error,
            result,
            duration_ms,
            num_turns,
            structured_output,
            ..
        } => {
            // When result is empty but structured_output is present
            // (Claude CLI with --json-schema), use the structured output.
            let effective_result = if result.is_empty() {
                if let Some(so) = structured_output {
                    log::debug!("Streaming Result.result is empty; using structured_output");
                    serde_json::to_string(so).unwrap_or_default()
                } else {
                    result.clone()
                }
            } else {
                result.clone()
            };
            Some(UnifiedEvent::Result {
                success: !is_error,
                message: Some(effective_result),
                duration_ms: Some(*duration_ms),
                num_turns: Some(*num_turns),
            })
        }
    }
}

#[cfg(test)]
#[path = "claude_tests.rs"]
mod tests;

impl Default for Claude {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for Claude {
    fn name(&self) -> &str {
        "claude"
    }

    fn default_model() -> &'static str {
        DEFAULT_MODEL
    }

    fn model_for_size(size: ModelSize) -> &'static str {
        match size {
            ModelSize::Small => "haiku",
            ModelSize::Medium => "sonnet",
            ModelSize::Large => "default",
        }
    }

    fn available_models() -> &'static [&'static str] {
        AVAILABLE_MODELS
    }

    crate::providers::common::impl_common_agent_setters!();

    fn set_skip_permissions(&mut self, skip: bool) {
        self.common.skip_permissions = skip;
    }

    crate::providers::common::impl_as_any!();

    async fn run(&self, prompt: Option<&str>) -> Result<Option<AgentOutput>> {
        self.execute(false, prompt).await
    }

    async fn run_interactive(&self, prompt: Option<&str>) -> Result<()> {
        self.execute(true, prompt).await?;
        Ok(())
    }

    async fn run_resume(&self, session_id: Option<&str>, _last: bool) -> Result<()> {
        let agent_args = self.build_resume_args(session_id);
        let mut cmd = self.make_command(agent_args);

        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let status = cmd
            .status()
            .await
            .context("Failed to execute 'claude' CLI. Is it installed and in PATH?")?;
        if !status.success() {
            return Err(crate::process::ProcessError {
                exit_code: status.code(),
                stderr: String::new(),
                agent_name: "Claude".to_string(),
            }
            .into());
        }
        Ok(())
    }

    async fn run_resume_with_prompt(
        &self,
        session_id: &str,
        prompt: &str,
    ) -> Result<Option<AgentOutput>> {
        log::debug!(
            "Claude resume with prompt: session={}, prompt={}",
            session_id,
            prompt
        );
        let in_sandbox = self.common.sandbox.is_some();
        let mut args = vec!["--print".to_string()];
        args.extend(["--resume".to_string(), session_id.to_string()]);
        args.extend(["--verbose", "--output-format", "json"].map(String::from));

        if self.common.skip_permissions && !in_sandbox {
            args.push("--dangerously-skip-permissions".to_string());
        }

        args.extend(["--model".to_string(), self.common.model.clone()]);

        for dir in &self.common.add_dirs {
            args.extend(["--add-dir".to_string(), dir.clone()]);
        }

        if let Some(ref schema) = self.json_schema {
            args.extend(["--json-schema".to_string(), schema.clone()]);
        }

        args.push(prompt.to_string());

        let mut cmd = self.make_command(args);

        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let output = cmd.output().await?;

        crate::process::handle_output(&output, "Claude")?;

        // Parse JSON output
        let json_str = String::from_utf8(output.stdout)?;
        log::debug!(
            "Parsing Claude resume JSON output ({} bytes)",
            json_str.len()
        );
        let claude_output: models::ClaudeOutput = serde_json::from_str(&json_str)
            .map_err(|e| anyhow::anyhow!("Failed to parse Claude resume JSON output: {}", e))?;

        let agent_output: AgentOutput = models::claude_output_to_agent_output(claude_output);
        Ok(Some(agent_output))
    }

    async fn cleanup(&self) -> Result<()> {
        Ok(())
    }
}
