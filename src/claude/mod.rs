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
}

impl Claude {
    pub fn new() -> Self {
        Self {
            system_prompt: String::new(),
            model: DEFAULT_MODEL.to_string(),
            root: None,
            skip_permissions: false,
            output_format: None,
        }
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

        let capture_json = !interactive
            && self
                .output_format
                .as_ref()
                .map_or(false, |f| f == "json" || f == "json-pretty" || f == "stream-json");

        if !interactive {
            cmd.arg("--print");

            // Add --verbose and --output-format for JSON outputs
            if let Some(ref format) = self.output_format {
                if format == "json" || format == "json-pretty" {
                    // For both json and json-pretty, pass "json" to claude CLI
                    // We handle the pretty printing in the wrapper
                    cmd.args(["--verbose", "--output-format", "json"]);
                } else if format == "stream-json" {
                    // Note: Not using --include-partial-messages because it adds stream_event types
                    // that would require additional parsing. The NDJSON format without it is sufficient
                    // for most use cases.
                    cmd.args(["--verbose", "--output-format", "stream-json"]);
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

        if let Some(p) = prompt {
            cmd.arg(p);
        }

        if capture_json {
            let is_stream_json = self
                .output_format
                .as_ref()
                .map_or(false, |f| f == "stream-json");

            if is_stream_json {
                // For stream-json, stream output directly to stdout line-by-line
                cmd.stdin(Stdio::inherit()).stderr(Stdio::inherit());
                cmd.stdout(Stdio::piped());

                let mut child = cmd.spawn()?;
                let stdout = child
                    .stdout
                    .take()
                    .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout"))?;

                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();

                // Stream each line to stdout as it arrives
                while let Some(line) = lines.next_line().await? {
                    println!("{}", line);
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
            // Normal mode - inherit stdout
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
