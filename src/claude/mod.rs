/// Claude agent implementation.
///
/// This module provides the Claude agent implementation, including:
/// - Agent trait implementation for executing Claude commands
/// - JSON output models for parsing Claude's verbose output
/// - Conversion to unified AgentOutput format
pub mod models;

use crate::agent::{Agent, ModelSize};
use crate::output::AgentOutput;
use anyhow::Result;
use async_trait::async_trait;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

pub const DEFAULT_MODEL: &str = "opus";

pub const AVAILABLE_MODELS: &[&str] = &["sonnet", "opus", "haiku"];

pub struct Claude {
    system_prompt: String,
    model: String,
    root: Option<String>,
    skip_permissions: bool,
    output_format: Option<String>,
    input_format: Option<String>,
    add_dirs: Vec<String>,
    worktree: Option<Option<String>>,
    capture_output: bool,
    verbose: bool,
    json_schema: Option<String>,
    session_id: Option<String>,
}

impl Claude {
    pub fn new() -> Self {
        Self {
            system_prompt: String::new(),
            model: DEFAULT_MODEL.to_string(),
            root: None,
            skip_permissions: false,
            output_format: None,
            input_format: None,
            add_dirs: Vec::new(),
            worktree: None,
            capture_output: false,
            verbose: false,
            json_schema: None,
            session_id: None,
        }
    }

    pub fn set_input_format(&mut self, format: Option<String>) {
        self.input_format = format;
    }

    pub fn set_worktree(&mut self, name: Option<String>) {
        self.worktree = Some(name);
    }

    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }

    pub fn set_json_schema(&mut self, schema: Option<String>) {
        self.json_schema = schema;
    }

    pub fn set_session_id(&mut self, id: String) {
        self.session_id = Some(id);
    }

    async fn execute(
        &self,
        interactive: bool,
        prompt: Option<&str>,
    ) -> Result<Option<AgentOutput>> {
        let mut cmd = Command::new("claude");

        if let Some(ref root) = self.root {
            cmd.current_dir(root);
        }

        // When capture_output is set (e.g. by auto-selector), use "json" format
        // so stdout is piped and parsed into AgentOutput
        let effective_output_format = if self.capture_output && self.output_format.is_none() {
            Some("json".to_string())
        } else {
            self.output_format.clone()
        };

        // Determine if we should capture structured output
        // Default to streaming unified output when no format is specified in print mode
        let capture_json = !interactive
            && effective_output_format
                .as_ref()
                .is_none_or(|f| f == "json" || f == "json-pretty" || f == "stream-json");

        if !interactive {
            cmd.arg("--print");

            // Add --verbose and --output-format for JSON outputs
            // Default to stream-json when no output format is specified
            match effective_output_format.as_deref() {
                Some("json") | Some("json-pretty") => {
                    // For both json and json-pretty, pass "json" to claude CLI
                    // We handle the pretty printing in the wrapper
                    cmd.args(["--verbose", "--output-format", "json"]);
                }
                Some("stream-json") | None => {
                    // Use stream-json for explicit stream-json or default (no output format)
                    // Note: Not using --include-partial-messages because it adds stream_event types
                    // that would require additional parsing. The NDJSON format without it is sufficient
                    // for most use cases.
                    cmd.args(["--verbose", "--output-format", "stream-json"]);
                }
                Some("native-json") => {
                    // Native JSON mode - output Claude's raw JSON without conversion
                    cmd.args(["--verbose", "--output-format", "json"]);
                }
                Some("text") => {
                    // Explicit text mode - don't add output format flags
                }
                _ => {
                    // Unknown format - ignore
                }
            }
        }

        if self.skip_permissions {
            cmd.arg("--dangerously-skip-permissions");
        }

        cmd.args(["--model", &self.model]);

        for dir in &self.add_dirs {
            cmd.args(["--add-dir", dir]);
        }

        if !self.system_prompt.is_empty() {
            cmd.args(["--append-system-prompt", &self.system_prompt]);
        }

        // Add input format if specified (only works with --print)
        if !interactive && let Some(ref input_fmt) = self.input_format {
            cmd.args(["--input-format", input_fmt]);
        }

        // Pass --worktree to claude binary (native support)
        if let Some(ref wt) = self.worktree {
            cmd.arg("--worktree");
            if let Some(name) = wt {
                cmd.arg(name);
            }
        }

        // Pass --session-id to claude binary
        if let Some(ref sid) = self.session_id {
            cmd.args(["--session-id", sid]);
        }

        // Pass --json-schema to claude binary (native support)
        if let Some(ref schema) = self.json_schema {
            cmd.args(["--json-schema", schema]);
        }

        if let Some(p) = prompt {
            cmd.arg(p);
        }

        // Check if we should pass through native JSON without conversion
        let is_native_json = effective_output_format.as_deref() == Some("native-json");

        if interactive {
            // Interactive mode - inherit all stdio
            cmd.stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());

            let status = cmd.status().await?;
            if !status.success() {
                anyhow::bail!("Claude command failed with status: {}", status);
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

                // Stream each line to stdout as it arrives
                while let Some(line) = lines.next_line().await? {
                    if format_as_text || format_as_json {
                        // Parse the NDJSON line and convert to unified format
                        match serde_json::from_str::<models::ClaudeEvent>(&line) {
                            Ok(claude_event) => {
                                // Convert individual event to unified format
                                if let Some(unified_event) =
                                    convert_claude_event_to_unified(&claude_event)
                                {
                                    if format_as_text {
                                        if self.verbose {
                                            // Verbose: format as beautiful text with icons
                                            if let Some(formatted) =
                                                crate::output::format_event_as_text(&unified_event)
                                            {
                                                println!("{}", formatted);
                                            }
                                        } else {
                                            // Default exec: plain text only from assistant messages
                                            if let crate::output::Event::AssistantMessage {
                                                ref content,
                                                ..
                                            } = unified_event
                                            {
                                                for block in content {
                                                    if let crate::output::ContentBlock::Text {
                                                        text,
                                                    } = block
                                                    {
                                                        print!("{}", text);
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        // Output as unified JSON (stream-json mode)
                                        if let Ok(json) = serde_json::to_string(&unified_event) {
                                            println!("{}", json);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                log::debug!(
                                    "Failed to parse streaming Claude event: {}. Line: {}",
                                    e,
                                    &line[..line.len().min(200)]
                                );
                            }
                        }
                    }
                }

                // Flush stdout and add trailing newline for plain text mode
                if format_as_text && !self.verbose {
                    use std::io::Write;
                    println!();
                    let _ = std::io::stdout().flush();
                }

                crate::process::wait_with_stderr(child).await?;

                // Return None to indicate output was streamed directly
                Ok(None)
            } else {
                // For json/json-pretty, capture all output then parse
                cmd.stdin(Stdio::inherit());
                cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

                let output = cmd.output().await?;

                // Handle stderr
                let stderr_text = String::from_utf8_lossy(&output.stderr);
                let stderr_text = stderr_text.trim();
                if !stderr_text.is_empty() {
                    for line in stderr_text.lines() {
                        crate::logging::log_to_file(&format!("[STDERR] {}", line));
                    }
                }

                if !output.status.success() {
                    if stderr_text.is_empty() {
                        anyhow::bail!("Claude command failed with status: {}", output.status);
                    } else {
                        anyhow::bail!("{}", stderr_text);
                    }
                }

                // Parse JSON output
                let json_str = String::from_utf8(output.stdout)?;
                log::debug!("Parsing Claude JSON output ({} bytes)", json_str.len());
                let claude_output: models::ClaudeOutput =
                    serde_json::from_str(&json_str).map_err(|e| {
                        log::debug!(
                            "Failed to parse Claude JSON output: {}. First 500 chars: {}",
                            e,
                            &json_str[..json_str.len().min(500)]
                        );
                        anyhow::anyhow!("Failed to parse Claude JSON output: {}", e)
                    })?;
                log::debug!("Parsed {} Claude events successfully", claude_output.len());

                // Convert to unified AgentOutput
                let agent_output: AgentOutput = claude_output.into();
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

/// Convert a single Claude event to a unified event format.
/// Returns None if the event doesn't map to a user-visible unified event.
fn convert_claude_event_to_unified(event: &models::ClaudeEvent) -> Option<crate::output::Event> {
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

        ClaudeEvent::Assistant { message, .. } => {
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
                    models::ContentBlock::Thinking { .. } => None,
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

            Some(UnifiedEvent::AssistantMessage { content, usage })
        }

        ClaudeEvent::User {
            message,
            tool_use_result,
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
                })
            } else {
                None
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
            ..
        } => Some(UnifiedEvent::Result {
            success: !is_error,
            message: Some(result.clone()),
            duration_ms: Some(*duration_ms),
            num_turns: Some(*num_turns),
        }),
    }
}

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
            ModelSize::Large => "opus",
        }
    }

    fn available_models() -> &'static [&'static str] {
        AVAILABLE_MODELS
    }

    fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    fn set_system_prompt(&mut self, prompt: String) {
        self.system_prompt = prompt;
    }

    fn get_model(&self) -> &str {
        &self.model
    }

    fn set_model(&mut self, model: String) {
        self.model = model;
    }

    fn set_root(&mut self, root: String) {
        self.root = Some(root);
    }

    fn set_skip_permissions(&mut self, skip: bool) {
        self.skip_permissions = skip;
    }

    fn set_output_format(&mut self, format: Option<String>) {
        self.output_format = format;
    }

    fn set_capture_output(&mut self, capture: bool) {
        self.capture_output = capture;
    }

    fn set_add_dirs(&mut self, dirs: Vec<String>) {
        self.add_dirs = dirs;
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    async fn run(&self, prompt: Option<&str>) -> Result<Option<AgentOutput>> {
        self.execute(false, prompt).await
    }

    async fn run_interactive(&self, prompt: Option<&str>) -> Result<()> {
        self.execute(true, prompt).await?;
        Ok(())
    }

    async fn run_resume(&self, session_id: Option<&str>, _last: bool) -> Result<()> {
        let mut cmd = Command::new("claude");

        if let Some(ref root) = self.root {
            cmd.current_dir(root);
        }

        if let Some(id) = session_id {
            cmd.args(["--resume", id]);
        } else {
            cmd.arg("--continue");
        }

        if self.skip_permissions {
            cmd.arg("--dangerously-skip-permissions");
        }

        cmd.args(["--model", &self.model]);

        for dir in &self.add_dirs {
            cmd.args(["--add-dir", dir]);
        }

        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let status = cmd.status().await?;
        if !status.success() {
            anyhow::bail!("Claude resume failed with status: {}", status);
        }
        Ok(())
    }

    async fn run_resume_with_prompt(
        &self,
        session_id: &str,
        prompt: &str,
    ) -> Result<Option<AgentOutput>> {
        let mut cmd = Command::new("claude");

        if let Some(ref root) = self.root {
            cmd.current_dir(root);
        }

        cmd.arg("--print");
        cmd.args(["--resume", session_id]);
        cmd.args(["--verbose", "--output-format", "json"]);

        if self.skip_permissions {
            cmd.arg("--dangerously-skip-permissions");
        }

        cmd.args(["--model", &self.model]);

        for dir in &self.add_dirs {
            cmd.args(["--add-dir", dir]);
        }

        if let Some(ref schema) = self.json_schema {
            cmd.args(["--json-schema", schema]);
        }

        cmd.arg(prompt);

        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let output = cmd.output().await?;

        // Handle stderr
        let stderr_text = String::from_utf8_lossy(&output.stderr);
        let stderr_text = stderr_text.trim();
        if !stderr_text.is_empty() {
            for line in stderr_text.lines() {
                crate::logging::log_to_file(&format!("[STDERR] {}", line));
            }
        }

        if !output.status.success() {
            if stderr_text.is_empty() {
                anyhow::bail!("Claude resume failed with status: {}", output.status);
            } else {
                anyhow::bail!("{}", stderr_text);
            }
        }

        // Parse JSON output
        let json_str = String::from_utf8(output.stdout)?;
        log::debug!(
            "Parsing Claude resume JSON output ({} bytes)",
            json_str.len()
        );
        let claude_output: models::ClaudeOutput = serde_json::from_str(&json_str)
            .map_err(|e| anyhow::anyhow!("Failed to parse Claude resume JSON output: {}", e))?;

        let agent_output: AgentOutput = claude_output.into();
        Ok(Some(agent_output))
    }

    async fn cleanup(&self) -> Result<()> {
        Ok(())
    }
}
