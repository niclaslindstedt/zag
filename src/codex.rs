use crate::agent::{Agent, ModelSize};
use crate::output::AgentOutput;
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
    output_format: Option<String>,
    add_dirs: Vec<String>,
}

impl Codex {
    pub fn new() -> Self {
        Self {
            system_prompt: String::new(),
            model: DEFAULT_MODEL.to_string(),
            root: None,
            skip_permissions: false,
            output_format: None,
            add_dirs: Vec::new(),
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

    pub async fn review(
        &self,
        uncommitted: bool,
        base: Option<&str>,
        commit: Option<&str>,
        title: Option<&str>,
    ) -> Result<()> {
        let mut cmd = Command::new("codex");
        cmd.arg("review");

        if uncommitted {
            cmd.arg("--uncommitted");
        }

        if let Some(b) = base {
            cmd.args(["--base", b]);
        }

        if let Some(c) = commit {
            cmd.args(["--commit", c]);
        }

        if let Some(t) = title {
            cmd.args(["--title", t]);
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

        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit());

        crate::process::run_with_captured_stderr(&mut cmd).await?;
        Ok(())
    }

    async fn execute(&self, interactive: bool, prompt: Option<&str>) -> Result<()> {
        if !self.system_prompt.is_empty() {
            self.write_agents_file().await?;
        }

        let mut cmd = Command::new("codex");

        if !interactive {
            cmd.args(["exec", "--skip-git-repo-check"]);
            if let Some(ref format) = self.output_format
                && format == "json"
            {
                cmd.arg("--json");
            }
        }

        if let Some(ref root) = self.root {
            cmd.args(["--cd", root]);
        }

        cmd.args(["--model", &self.model]);

        for dir in &self.add_dirs {
            cmd.args(["--add-dir", dir]);
        }

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
            .stdout(Stdio::inherit());

        if interactive {
            cmd.stderr(Stdio::inherit());
            let status = cmd.status().await?;
            if !status.success() {
                anyhow::bail!("Codex command failed with status: {}", status);
            }
        } else {
            crate::process::run_with_captured_stderr(&mut cmd).await?;
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

    fn set_output_format(&mut self, format: Option<String>) {
        self.output_format = format;
    }

    fn set_add_dirs(&mut self, dirs: Vec<String>) {
        self.add_dirs = dirs;
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    async fn run(&self, prompt: Option<&str>) -> Result<Option<AgentOutput>> {
        self.execute(false, prompt).await?;
        Ok(None)
    }

    async fn run_interactive(&self, prompt: Option<&str>) -> Result<()> {
        self.execute(true, prompt).await
    }

    async fn run_resume(&self, session_id: Option<&str>, last: bool) -> Result<()> {
        let mut cmd = Command::new("codex");
        cmd.arg("resume");

        if let Some(id) = session_id {
            cmd.arg(id);
        } else if last {
            cmd.arg("--last");
        }

        if let Some(ref root) = self.root {
            cmd.args(["--cd", root]);
        }

        cmd.args(["--model", &self.model]);

        for dir in &self.add_dirs {
            cmd.args(["--add-dir", dir]);
        }

        if self.skip_permissions {
            cmd.args([
                "--dangerously-bypass-approvals-and-sandbox",
                "--sandbox",
                "danger-full-access",
            ]);
        }

        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let status = cmd.status().await?;
        if !status.success() {
            anyhow::bail!("Codex resume failed with status: {}", status);
        }
        Ok(())
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
