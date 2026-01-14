use crate::agent::{Agent, ModelSize};
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;

pub const DEFAULT_MODEL: &str = "gpt-5.2-codex";

pub const AVAILABLE_MODELS: &[&str] = &[
    "gpt-5.2-codex",
    "gpt-5.1-codex-max",
    "gpt-5.1-codex-mini",
    "gpt-5.2",
];

pub struct Codex {
    system_prompt: String,
    model: String,
    root: Option<String>,
    skip_permissions: bool,
}

impl Codex {
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

    async fn write_agents_file(&self) -> Result<()> {
        let base = self.get_base_path();
        let codex_dir = base.join(".codex");
        fs::create_dir_all(&codex_dir).await?;
        fs::write(codex_dir.join("AGENTS.md"), &self.system_prompt).await?;
        Ok(())
    }

    async fn execute(&self, interactive: bool, prompt: Option<&str>) -> Result<()> {
        if !self.system_prompt.is_empty() {
            self.write_agents_file().await?;
        }

        let mut cmd = Command::new("codex");

        if !interactive {
            cmd.args(["exec", "--skip-git-repo-check", "--json"]);
        }

        if let Some(ref root) = self.root {
            cmd.args(["--cd", root]);
        }

        cmd.args(["--model", &self.model]);

        if self.skip_permissions {
            cmd.args([
                "--dangerously-bypass-approvals-and-sandbox",
                "--sandbox",
                "danger-full-access",
            ]);
        }

        if let Some(p) = prompt {
            cmd.arg(p);
        }

        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let status = cmd.status().await?;
        if !status.success() {
            anyhow::bail!("Codex command failed with status: {}", status);
        }
        Ok(())
    }
}

impl Default for Codex {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for Codex {
    fn name(&self) -> &str {
        "codex"
    }

    fn default_model() -> &'static str {
        DEFAULT_MODEL
    }

    fn model_for_size(size: ModelSize) -> &'static str {
        match size {
            ModelSize::Small => "gpt-5.1-codex-mini",
            ModelSize::Medium => "gpt-5.2-codex",
            ModelSize::Large => "gpt-5.1-codex-max",
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
        let base = self.get_base_path();
        let codex_dir = base.join(".codex");
        let agents_file = codex_dir.join("AGENTS.md");

        if agents_file.exists() {
            fs::remove_file(&agents_file).await?;
        }

        if codex_dir.exists()
            && fs::read_dir(&codex_dir)
                .await?
                .next_entry()
                .await?
                .is_none()
        {
            fs::remove_dir(&codex_dir).await?;
        }

        Ok(())
    }
}
