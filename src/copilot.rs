use crate::agent::{Agent, ModelSize};
use crate::output::AgentOutput;
use crate::sandbox::SandboxConfig;
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;

pub const DEFAULT_MODEL: &str = "claude-sonnet-4.5";

pub const AVAILABLE_MODELS: &[&str] = &[
    "claude-sonnet-4.5",
    "claude-haiku-4.5",
    "claude-opus-4.5",
    "claude-sonnet-4",
    "gpt-5.1-codex-max",
    "gpt-5.1-codex",
    "gpt-5.2",
    "gpt-5.1",
    "gpt-5",
    "gpt-5.1-codex-mini",
    "gpt-5-mini",
    "gpt-4.1",
    "gemini-3-pro-preview",
];

pub struct Copilot {
    system_prompt: String,
    model: String,
    root: Option<String>,
    skip_permissions: bool,
    output_format: Option<String>,
    add_dirs: Vec<String>,
    capture_output: bool,
    sandbox: Option<SandboxConfig>,
}

impl Copilot {
    pub fn new() -> Self {
        Self {
            system_prompt: String::new(),
            model: DEFAULT_MODEL.to_string(),
            root: None,
            skip_permissions: false,
            output_format: None,
            add_dirs: Vec::new(),
            capture_output: false,
            sandbox: None,
        }
    }

    fn get_base_path(&self) -> &Path {
        self.root.as_ref().map(Path::new).unwrap_or(Path::new("."))
    }

    async fn write_instructions_file(&self) -> Result<()> {
        let base = self.get_base_path();
        let instructions_dir = base.join(".github/instructions/agent");
        fs::create_dir_all(&instructions_dir).await?;
        fs::write(
            instructions_dir.join("agent.instructions.md"),
            &self.system_prompt,
        )
        .await?;
        Ok(())
    }

    /// Build the argument list for a run/exec invocation.
    fn build_run_args(&self, interactive: bool, prompt: Option<&str>) -> Vec<String> {
        let mut args = Vec::new();

        // In non-interactive mode, --allow-all-tools is required
        if !interactive || self.skip_permissions {
            args.push("--allow-all-tools".to_string());
        }

        if !self.model.is_empty() {
            args.extend(["--model".to_string(), self.model.clone()]);
        }

        for dir in &self.add_dirs {
            args.extend(["--add-dir".to_string(), dir.clone()]);
        }

        match (interactive, prompt) {
            (true, Some(p)) => args.extend(["-i".to_string(), p.to_string()]),
            (false, Some(p)) => args.extend(["-p".to_string(), p.to_string()]),
            _ => {}
        }

        args
    }

    /// Create a `Command` either directly or wrapped in sandbox.
    fn make_command(&self, agent_args: Vec<String>) -> Command {
        if let Some(ref sb) = self.sandbox {
            let std_cmd = crate::sandbox::build_sandbox_command(sb, agent_args);
            Command::from(std_cmd)
        } else {
            let mut cmd = Command::new("copilot");
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
        // Output format flags are not supported by Copilot
        if self.output_format.is_some() {
            anyhow::bail!(
                "Copilot does not support the --output flag. Remove the flag and try again."
            );
        }

        if !self.system_prompt.is_empty() {
            self.write_instructions_file().await?;
        }

        let agent_args = self.build_run_args(interactive, prompt);
        let mut cmd = self.make_command(agent_args);

        if interactive {
            cmd.stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
            let status = cmd.status().await?;
            if !status.success() {
                anyhow::bail!("Copilot command failed with status: {}", status);
            }
            Ok(None)
        } else if self.capture_output {
            let text = crate::process::run_captured(&mut cmd, "Copilot").await?;
            Ok(Some(AgentOutput::from_text("copilot", &text)))
        } else {
            cmd.stdin(Stdio::inherit()).stdout(Stdio::inherit());
            crate::process::run_with_captured_stderr(&mut cmd).await?;
            Ok(None)
        }
    }
}

#[cfg(test)]
#[path = "copilot_tests.rs"]
mod tests;

impl Default for Copilot {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for Copilot {
    fn name(&self) -> &str {
        "copilot"
    }

    fn default_model() -> &'static str {
        DEFAULT_MODEL
    }

    fn model_for_size(size: ModelSize) -> &'static str {
        match size {
            ModelSize::Small => "claude-haiku-4.5",
            ModelSize::Medium => "claude-sonnet-4.5",
            ModelSize::Large => "claude-opus-4.5",
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

    fn set_add_dirs(&mut self, dirs: Vec<String>) {
        self.add_dirs = dirs;
    }

    fn set_capture_output(&mut self, capture: bool) {
        self.capture_output = capture;
    }

    fn set_sandbox(&mut self, config: SandboxConfig) {
        self.sandbox = Some(config);
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
        let mut args = vec!["--resume".to_string()];

        if self.skip_permissions {
            args.push("--allow-all-tools".to_string());
        }

        if !self.model.is_empty() {
            args.extend(["--model".to_string(), self.model.clone()]);
        }

        for dir in &self.add_dirs {
            args.extend(["--add-dir".to_string(), dir.clone()]);
        }

        let mut cmd = self.make_command(args);

        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let status = cmd.status().await?;
        if !status.success() {
            anyhow::bail!("Copilot resume failed with status: {}", status);
        }
        Ok(())
    }

    async fn cleanup(&self) -> Result<()> {
        let base = self.get_base_path();
        let instructions_file = base.join(".github/instructions/agent/agent.instructions.md");

        if instructions_file.exists() {
            fs::remove_file(&instructions_file).await?;
        }

        // Clean up empty directories
        let agent_dir = base.join(".github/instructions/agent");
        if agent_dir.exists()
            && fs::read_dir(&agent_dir)
                .await?
                .next_entry()
                .await?
                .is_none()
        {
            fs::remove_dir(&agent_dir).await?;
        }

        let instructions_dir = base.join(".github/instructions");
        if instructions_dir.exists()
            && fs::read_dir(&instructions_dir)
                .await?
                .next_entry()
                .await?
                .is_none()
        {
            fs::remove_dir(&instructions_dir).await?;
        }

        let github_dir = base.join(".github");
        if github_dir.exists()
            && fs::read_dir(&github_dir)
                .await?
                .next_entry()
                .await?
                .is_none()
        {
            fs::remove_dir(&github_dir).await?;
        }

        Ok(())
    }
}
