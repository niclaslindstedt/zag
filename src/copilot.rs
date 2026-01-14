use crate::agent::{Agent, ModelSize};
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
}

impl Copilot {
    pub fn new() -> Self {
        Self {
            system_prompt: String::new(),
            model: DEFAULT_MODEL.to_string(),
            root: None,
            skip_permissions: false,
            output_format: None,
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

    async fn execute(&self, interactive: bool, prompt: Option<&str>) -> Result<()> {
        // Check if output format is set (not supported by Copilot)
        if self.output_format.is_some() {
            anyhow::bail!(
                "Copilot does not support the --output flag. Remove the flag and try again."
            );
        }

        if !self.system_prompt.is_empty() {
            self.write_instructions_file().await?;
        }

        let mut cmd = Command::new("copilot");

        if let Some(ref root) = self.root {
            cmd.current_dir(root);
        }

        // In non-interactive mode, --allow-all-tools is required
        if !interactive || self.skip_permissions {
            cmd.arg("--allow-all-tools");
        }

        if !self.model.is_empty() {
            cmd.args(["--model", &self.model]);
        }

        match (interactive, prompt) {
            (true, Some(p)) => cmd.args(["-i", p]),
            (true, None) => &mut cmd, // Interactive is default for copilot CLI
            (false, Some(p)) => cmd.args(["-p", p]),
            (false, None) => &mut cmd, // No prompt in non-interactive mode
        };

        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let status = cmd.status().await?;
        if !status.success() {
            anyhow::bail!("Copilot command failed with status: {}", status);
        }
        Ok(())
    }
}

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

    async fn run(&self, prompt: Option<&str>) -> Result<()> {
        self.execute(false, prompt).await
    }

    async fn run_interactive(&self, prompt: Option<&str>) -> Result<()> {
        self.execute(true, prompt).await
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
