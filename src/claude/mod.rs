pub mod logs;
/// Claude agent implementation.
///
/// This module provides the Claude agent implementation, including:
/// - Agent trait implementation for executing Claude commands
/// - JSON output models for parsing Claude's verbose output
/// - Conversion to unified AgentOutput format
pub mod models;

use crate::agent::{Agent, ModelSize};
use crate::output::AgentOutput;
use crate::sandbox::SandboxConfig;
use anyhow::Result;
use async_trait::async_trait;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

pub const DEFAULT_MODEL: &str = "default";

pub const AVAILABLE_MODELS: &[&str] = &[
    "default",
    "sonnet",
    "sonnet-4.6",
    "opus",
    "opus-4.6",
    "haiku",
    "haiku-4.5",
];

pub struct Claude {
    system_prompt: String,
    model: String,
    root: Option<String>,
    session_id: Option<String>,
    skip_permissions: bool,
    output_format: Option<String>,
    input_format: Option<String>,
    add_dirs: Vec<String>,
    capture_output: bool,
    verbose: bool,
    json_schema: Option<String>,
    sandbox: Option<SandboxConfig>,
}

impl Claude {
    pub fn new() -> Self {
        Self {
            system_prompt: String::new(),
            model: DEFAULT_MODEL.to_string(),
            root: None,
            session_id: None,
            skip_permissions: false,
            output_format: None,
            input_format: None,
            add_dirs: Vec::new(),
            capture_output: false,
            verbose: false,
            json_schema: None,
            sandbox: None,
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

    /// Build the argument list for a run/exec invocation.
    fn build_run_args(
        &self,
        interactive: bool,
        prompt: Option<&str>,
        effective_output_format: &Option<String>,
    ) -> Vec<String> {
        let mut args = Vec::new();
        let in_sandbox = self.sandbox.is_some();

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
        if self.skip_permissions && !in_sandbox {
            args.push("--dangerously-skip-permissions".to_string());
        }

        args.extend(["--model".to_string(), self.model.clone()]);

        if interactive && let Some(session_id) = &self.session_id {
            args.extend(["--session-id".to_string(), session_id.clone()]);
        }

        for dir in &self.add_dirs {
            args.extend(["--add-dir".to_string(), dir.clone()]);
        }

        if !self.system_prompt.is_empty() {
            args.extend([
                "--append-system-prompt".to_string(),
                self.system_prompt.clone(),
            ]);
        }

        if !interactive && let Some(ref input_fmt) = self.input_format {
            args.extend(["--input-format".to_string(), input_fmt.clone()]);
        }

        if let Some(ref schema) = self.json_schema {
            args.extend(["--json-schema".to_string(), schema.clone()]);
        }

        if let Some(p) = prompt {
            args.push(p.to_string());
        }

        args
    }

    /// Build the argument list for a resume invocation.
    fn build_resume_args(&self, session_id: Option<&str>) -> Vec<String> {
        let mut args = Vec::new();
        let in_sandbox = self.sandbox.is_some();

        if let Some(id) = session_id {
            args.extend(["--resume".to_string(), id.to_string()]);
        } else {
            args.push("--continue".to_string());
        }

        if self.skip_permissions && !in_sandbox {
            args.push("--dangerously-skip-permissions".to_string());
        }

        args.extend(["--model".to_string(), self.model.clone()]);

        for dir in &self.add_dirs {
            args.extend(["--add-dir".to_string(), dir.clone()]);
        }

        args
    }

    /// Create a `Command` either directly or wrapped in sandbox.
    fn make_command(&self, agent_args: Vec<String>) -> Command {
        if let Some(ref sb) = self.sandbox {
            let std_cmd = crate::sandbox::build_sandbox_command(sb, agent_args);
            Command::from(std_cmd)
        } else {
            let mut cmd = Command::new("claude");
            if let Some(ref root) = self.root {
                cmd.current_dir(root);
            }
            cmd.args(&agent_args);
            cmd
        }
    }

    async fn execute(
        &self,
        interactive: bool,
        prompt: Option<&str>,
    ) -> Result<Option<AgentOutput>> {
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

        let agent_args = self.build_run_args(interactive, prompt, &effective_output_format);
        log::debug!("Claude command: claude {}", agent_args.join(" "));
        if !self.system_prompt.is_empty() {
            log::debug!("Claude system prompt: {}", self.system_prompt);
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

                crate::process::handle_output(&output, "Claude")?;

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

    fn set_sandbox(&mut self, config: SandboxConfig) {
        self.sandbox = Some(config);
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
        let agent_args = self.build_resume_args(session_id);
        let mut cmd = self.make_command(agent_args);

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
        log::debug!(
            "Claude resume with prompt: session={}, prompt={}",
            session_id,
            prompt
        );
        let in_sandbox = self.sandbox.is_some();
        let mut args = vec!["--print".to_string()];
        args.extend(["--resume".to_string(), session_id.to_string()]);
        args.extend(["--verbose", "--output-format", "json"].map(String::from));

        if self.skip_permissions && !in_sandbox {
            args.push("--dangerously-skip-permissions".to_string());
        }

        args.extend(["--model".to_string(), self.model.clone()]);

        for dir in &self.add_dirs {
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
