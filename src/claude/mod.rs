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
        }
    }

    pub fn set_input_format(&mut self, format: Option<String>) {
        self.input_format = format;
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

        // Determine if we should capture structured output
        // Default to streaming unified output when no format is specified in print mode
        let capture_json = !interactive
            && self
                .output_format
                .as_ref()
                .map_or(true, |f| f == "json" || f == "json-pretty" || f == "stream-json");

        if !interactive {
            cmd.arg("--print");

            // Add --verbose and --output-format for JSON outputs
            // Default to stream-json when no output format is specified
            match self.output_format.as_deref() {
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

        if !self.system_prompt.is_empty() {
            cmd.args(["--append-system-prompt", &self.system_prompt]);
        }

        // Add input format if specified (only works with --print)
        if !interactive {
            if let Some(ref input_fmt) = self.input_format {
                cmd.args(["--input-format", input_fmt]);
            }
        }

        if let Some(p) = prompt {
            cmd.arg(p);
        }

        // Check if we should pass through native JSON without conversion
        let is_native_json = self.output_format.as_deref() == Some("native-json");

        if is_native_json {
            // Native JSON mode - pass through Claude's raw JSON output
            cmd.stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());

            let status = cmd.status().await?;
            if !status.success() {
                anyhow::bail!("Claude command failed with status: {}", status);
            }
            Ok(None)
        } else if capture_json {
            let output_format = self.output_format.as_deref();
            let is_streaming = output_format == Some("stream-json") || output_format.is_none();

            if is_streaming {
                // For stream-json or default (None), stream output and convert to unified format
                cmd.stdin(Stdio::inherit()).stderr(Stdio::inherit());
                cmd.stdout(Stdio::piped());

                let mut child = cmd.spawn()?;
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
                        if let Ok(claude_event) = serde_json::from_str::<models::ClaudeEvent>(&line) {
                            // Convert individual event to unified format
                            if let Some(unified_event) = convert_claude_event_to_unified(&claude_event) {
                                if format_as_text {
                                    // Format as beautiful text
                                    if let Some(formatted) = crate::output::format_event_as_text(&unified_event) {
                                        println!("{}", formatted);
                                    }
                                } else {
                                    // Output as unified JSON (stream-json mode)
                                    if let Ok(json) = serde_json::to_string(&unified_event) {
                                        println!("{}", json);
                                    }
                                }
                            }
                        }
                        // If parsing fails, silently skip (could be a malformed line)
                    }
                }

                let status = child.wait().await?;
                if !status.success() {
                    anyhow::bail!("Claude command failed with status: {}", status);
                }

                // Return None to indicate output was streamed directly
                Ok(None)
            } else {
                // For json/json-pretty, capture all output then parse
                cmd.stdin(Stdio::inherit()).stderr(Stdio::inherit());
                cmd.stdout(Stdio::piped());

                let output = cmd.output().await?;
                if !output.status.success() {
                    anyhow::bail!("Claude command failed with status: {}", output.status);
                }

                // Parse JSON output
                let json_str = String::from_utf8(output.stdout)?;
                let claude_output: models::ClaudeOutput = serde_json::from_str(&json_str)
                    .map_err(|e| anyhow::anyhow!("Failed to parse Claude JSON output: {}", e))?;

                // Convert to unified AgentOutput
                let agent_output: AgentOutput = claude_output.into();
                Ok(Some(agent_output))
            }
        } else {
            // Explicit text mode - inherit stdout (pass through)
            cmd.stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());

            let status = cmd.status().await?;
            if !status.success() {
                anyhow::bail!("Claude command failed with status: {}", status);
            }
            Ok(None)
        }
    }
}

/// Convert a single Claude event to a unified event format.
/// Returns None if the event doesn't map to a user-visible unified event.
fn convert_claude_event_to_unified(event: &models::ClaudeEvent) -> Option<crate::output::Event> {
    use crate::output::{ContentBlock as UnifiedContentBlock, Event as UnifiedEvent, ToolResult, Usage as UnifiedUsage};
    use models::ClaudeEvent;

    match event {
        ClaudeEvent::System {
            model,
            tools,
            cwd,
            ..
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
                .map(|block| match block {
                    models::ContentBlock::Text { text } => UnifiedContentBlock::Text {
                        text: text.clone(),
                    },
                    models::ContentBlock::ToolUse { id, name, input } => {
                        UnifiedContentBlock::ToolUse {
                            id: id.clone(),
                            name: name.clone(),
                            input: input.clone(),
                        }
                    }
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

        ClaudeEvent::User { message, tool_use_result, .. } => {
            // For streaming, we can't easily look up tool names from previous events
            // So we'll use "unknown" for the tool name in streaming mode
            // This is a limitation of streaming individual events
            if let Some(result_block) = message.content.first() {
                let tool_result = ToolResult {
                    success: !result_block.is_error,
                    output: if !result_block.is_error {
                        Some(result_block.content.clone())
                    } else {
                        None
                    },
                    error: if result_block.is_error {
                        Some(result_block.content.clone())
                    } else {
                        None
                    },
                    data: tool_use_result.clone(),
                };

                Some(UnifiedEvent::ToolExecution {
                    tool_name: "unknown".to_string(),
                    tool_id: result_block.tool_use_id.clone(),
                    input: serde_json::Value::Null,
                    result: tool_result,
                })
            } else {
                None
            }
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

    async fn cleanup(&self) -> Result<()> {
        Ok(())
    }
}
