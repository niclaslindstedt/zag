use crate::agent::{Agent, ModelSize};
use anyhow::Result;
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

pub const DEFAULT_MODEL: &str = "opus";

pub const AVAILABLE_MODELS: &[&str] = &["sonnet", "opus", "haiku"];

pub struct Claude {
    system_prompt: String,
    model: String,
    root: Option<String>,
    skip_permissions: bool,
}

impl Claude {
    pub fn new() -> Self {
        Self {
            system_prompt: String::new(),
            model: DEFAULT_MODEL.to_string(),
            root: None,
            skip_permissions: false,
        }
    }

    async fn execute(&self, interactive: bool, prompt: Option<&str>) -> Result<()> {
        let mut cmd = Command::new("claude");

        if let Some(ref root) = self.root {
            cmd.current_dir(root);
        }

        if !interactive {
            cmd.args(["--print", "--verbose", "--output-format", "json"]);
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

        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let status = cmd.status().await?;
        if !status.success() {
            anyhow::bail!("Claude command failed with status: {}", status);
        }
        Ok(())
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

    async fn run(&self, prompt: Option<&str>) -> Result<()> {
        self.execute(false, prompt).await
    }

    async fn run_interactive(&self, prompt: Option<&str>) -> Result<()> {
        self.execute(true, prompt).await
    }

    async fn cleanup(&self) -> Result<()> {
        Ok(())
    }
}
