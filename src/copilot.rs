use crate::agent::Agent;
use crate::process::wait_with_signal_handling;
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;

pub const DEFAULT_MODEL: &str = "claude-sonnet-4.5";

pub const AVAILABLE_MODELS: &[(&str, &str)] = &[
    ("claude-sonnet-4.5", "Claude Sonnet 4.5"),
    ("claude-haiku-4.5", "Claude Haiku 4.5"),
    ("claude-opus-4.5", "Claude Opus 4.5"),
    ("claude-sonnet-4", "Claude Sonnet 4"),
    ("gpt-5.1-codex-max", "GPT 5.1 Codex Max"),
    ("gpt-5.1-codex", "GPT 5.1 Codex"),
    ("gpt-5.2", "GPT 5.2"),
    ("gpt-5.1", "GPT 5.1"),
    ("gpt-5", "GPT 5"),
    ("gpt-5.1-codex-mini", "GPT 5.1 Codex Mini"),
    ("gpt-5-mini", "GPT 5 Mini"),
    ("gpt-4.1", "GPT 4.1"),
    ("gemini-3-pro-preview", "Gemini 3 Pro Preview"),
];

pub struct Copilot {
    system_prompt: String,
    model: String,
    root: Option<String>,
    skip_permissions: bool,
}

impl Copilot {
    pub fn new() -> Self {
        Self {
            system_prompt: String::new(),
            model: DEFAULT_MODEL.to_string(),
            root: None,
            skip_permissions: false,
        }
    }

    fn get_base_path(&self) -> &Path {
        self.root.as_ref().map(Path::new).unwrap_or(Path::new("."))
    }

    async fn write_instructions_file(&self) -> Result<()> {
        let base = self.get_base_path();
        let instructions_dir = base.join(".github/instructions/agent");
        fs::create_dir_all(&instructions_dir).await?;
        fs::write(instructions_dir.join("agent.instructions.md"), &self.system_prompt).await?;
        Ok(())
    }

    async fn execute(&self, interactive: bool, prompt: &str) -> Result<()> {
        if !self.system_prompt.is_empty() {
            self.write_instructions_file().await?;
        }

        let mut cmd = Command::new("copilot");

        if let Some(ref root) = self.root {
            cmd.current_dir(root);
        }

        if self.skip_permissions {
            cmd.arg("--allow-all-tools");
        }

        if !self.model.is_empty() {
            cmd.args(["--model", &self.model]);
        }

        if interactive {
            cmd.args(["-i", prompt]);
        } else {
            cmd.args(["-p", prompt]);
        }

        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let child = cmd.spawn()?;
        wait_with_signal_handling(child).await
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

    fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    fn set_system_prompt(&mut self, prompt: String) {
        self.system_prompt = prompt;
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

    async fn run(&self, prompt: &str) -> Result<()> {
        self.execute(false, prompt).await
    }

    async fn run_interactive(&self, prompt: &str) -> Result<()> {
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
        if agent_dir.exists() && fs::read_dir(&agent_dir).await?.next_entry().await?.is_none() {
            fs::remove_dir(&agent_dir).await?;
        }

        let instructions_dir = base.join(".github/instructions");
        if instructions_dir.exists() && fs::read_dir(&instructions_dir).await?.next_entry().await?.is_none() {
            fs::remove_dir(&instructions_dir).await?;
        }

        let github_dir = base.join(".github");
        if github_dir.exists() && fs::read_dir(&github_dir).await?.next_entry().await?.is_none() {
            fs::remove_dir(&github_dir).await?;
        }

        Ok(())
    }
}
