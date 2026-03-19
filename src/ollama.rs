use crate::agent::{Agent, ModelSize};
use crate::output::AgentOutput;
use crate::sandbox::SandboxConfig;
use anyhow::Result;
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

pub const DEFAULT_MODEL: &str = "qwen3.5";
pub const DEFAULT_SIZE: &str = "9b";

pub const AVAILABLE_SIZES: &[&str] = &["0.8b", "2b", "4b", "9b", "27b", "35b", "122b"];

pub struct Ollama {
    system_prompt: String,
    model: String,
    size: String,
    root: Option<String>,
    skip_permissions: bool,
    output_format: Option<String>,
    add_dirs: Vec<String>,
    capture_output: bool,
    sandbox: Option<SandboxConfig>,
}

impl Ollama {
    pub fn new() -> Self {
        Self {
            system_prompt: String::new(),
            model: DEFAULT_MODEL.to_string(),
            size: DEFAULT_SIZE.to_string(),
            root: None,
            skip_permissions: false,
            output_format: None,
            add_dirs: Vec::new(),
            capture_output: false,
            sandbox: None,
        }
    }

    pub fn set_size(&mut self, size: String) {
        self.size = size;
    }

    /// Get the display string for the model (e.g., "qwen3.5:9b").
    pub fn display_model(&self) -> String {
        self.model_tag()
    }

    /// Get the full model tag (e.g., "qwen3.5:9b").
    fn model_tag(&self) -> String {
        format!("{}:{}", self.model, self.size)
    }

    /// Build the argument list for a run invocation.
    fn build_run_args(&self, interactive: bool, prompt: Option<&str>) -> Vec<String> {
        let mut args = vec!["run".to_string()];

        if !self.system_prompt.is_empty() {
            args.extend(["--system".to_string(), self.system_prompt.clone()]);
        }

        if let Some(ref format) = self.output_format
            && format == "json"
        {
            args.extend(["--format".to_string(), "json".to_string()]);
        }

        if !interactive {
            // --nowordwrap for clean piped output
            args.push("--nowordwrap".to_string());
        }

        args.push(self.model_tag());

        if let Some(p) = prompt {
            args.push(p.to_string());
        }

        args
    }

    /// Create a `Command` either directly or wrapped in sandbox.
    fn make_command(&self, agent_args: Vec<String>) -> Command {
        if let Some(ref sb) = self.sandbox {
            // For ollama in sandbox, we use the shell template:
            // docker sandbox run shell <workspace> -- -c "ollama run ..."
            let shell_cmd = format!(
                "ollama {}",
                agent_args
                    .iter()
                    .map(|a| shell_escape(a))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            let mut std_cmd = std::process::Command::new("docker");
            std_cmd.args([
                "sandbox",
                "run",
                "--name",
                &sb.name,
                &sb.template,
                &sb.workspace,
                "--",
                "-c",
                &shell_cmd,
            ]);
            log::debug!(
                "Sandbox command: docker sandbox run --name {} {} {} -- -c {:?}",
                sb.name,
                sb.template,
                sb.workspace,
                shell_cmd
            );
            Command::from(std_cmd)
        } else {
            let mut cmd = Command::new("ollama");
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
        let agent_args = self.build_run_args(interactive, prompt);
        let mut cmd = self.make_command(agent_args);

        if interactive {
            cmd.stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
            let status = cmd.status().await?;
            if !status.success() {
                anyhow::bail!("Ollama command failed with status: {}", status);
            }
            Ok(None)
        } else if self.capture_output {
            let text = crate::process::run_captured(&mut cmd, "Ollama").await?;
            Ok(Some(AgentOutput::from_text("ollama", &text)))
        } else {
            cmd.stdin(Stdio::inherit()).stdout(Stdio::inherit());
            crate::process::run_with_captured_stderr(&mut cmd).await?;
            Ok(None)
        }
    }

    /// Resolve a size alias to the appropriate parameter size.
    pub fn size_for_model_size(size: ModelSize) -> &'static str {
        match size {
            ModelSize::Small => "2b",
            ModelSize::Medium => "9b",
            ModelSize::Large => "35b",
        }
    }
}

/// Escape a string for shell use. Wraps in single quotes if it contains special chars.
fn shell_escape(s: &str) -> String {
    if s.contains(' ')
        || s.contains('\'')
        || s.contains('"')
        || s.contains('\\')
        || s.contains('$')
        || s.contains('`')
        || s.contains('!')
    {
        format!("'{}'", s.replace('\'', "'\\''"))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
#[path = "ollama_tests.rs"]
mod tests;

impl Default for Ollama {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for Ollama {
    fn name(&self) -> &str {
        "ollama"
    }

    fn default_model() -> &'static str
    where
        Self: Sized,
    {
        DEFAULT_MODEL
    }

    fn model_for_size(size: ModelSize) -> &'static str
    where
        Self: Sized,
    {
        // For ollama, model_for_size returns the size parameter, not the model name
        Self::size_for_model_size(size)
    }

    fn available_models() -> &'static [&'static str]
    where
        Self: Sized,
    {
        // Ollama accepts any model — return common sizes for validation/help
        AVAILABLE_SIZES
    }

    /// Ollama uses open model names — skip strict validation.
    fn validate_model(_model: &str, _agent_name: &str) -> Result<()>
    where
        Self: Sized,
    {
        Ok(())
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

    fn set_skip_permissions(&mut self, _skip: bool) {
        // Ollama runs locally — no permission concept
        self.skip_permissions = true;
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

    async fn run_resume(&self, _session_id: Option<&str>, _last: bool) -> Result<()> {
        anyhow::bail!("Ollama does not support session resume")
    }

    async fn cleanup(&self) -> Result<()> {
        Ok(())
    }
}
