use crate::agent::{Agent, ModelSize};
use crate::output::AgentOutput;
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;

pub const DEFAULT_MODEL: &str = "auto";

pub const AVAILABLE_MODELS: &[&str] = &[
    "auto",
    "gemini-3-pro-preview",
    "gemini-3-flash-preview",
    "gemini-2.5-pro",
    "gemini-2.5-flash",
    "gemini-2.5-flash-lite",
];

pub struct Gemini {
    system_prompt: String,
    model: String,
    root: Option<String>,
    skip_permissions: bool,
    output_format: Option<String>,
    add_dirs: Vec<String>,
}

impl Gemini {
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

    async fn write_system_file(&self) -> Result<()> {
        let base = self.get_base_path();
        let gemini_dir = base.join(".gemini");
        fs::create_dir_all(&gemini_dir).await?;
        fs::write(gemini_dir.join("system.md"), &self.system_prompt).await?;
        Ok(())
    }

    async fn execute(&self, interactive: bool, prompt: Option<&str>) -> Result<()> {
        if !self.system_prompt.is_empty() {
            self.write_system_file().await?;
        }

        let mut cmd = Command::new("gemini");

        if let Some(ref root) = self.root {
            cmd.current_dir(root);
        }

        if !self.system_prompt.is_empty() {
            cmd.env("GEMINI_SYSTEM_MD", "true");
        }

        if self.skip_permissions {
            cmd.args(["--approval-mode", "yolo"]);
        }

        if !self.model.is_empty() && self.model != "auto" {
            cmd.args(["--model", &self.model]);
        }

        for dir in &self.add_dirs {
            cmd.args(["--include-directories", dir]);
        }

        if !interactive {
            if let Some(ref format) = self.output_format {
                cmd.args(["--output-format", format]);
            }
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
                anyhow::bail!("Gemini command failed with status: {}", status);
            }
        } else {
            crate::process::run_with_captured_stderr(&mut cmd).await?;
        }

        Ok(())
    }
}

impl Default for Gemini {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for Gemini {
    fn name(&self) -> &str {
        "gemini"
    }

    fn default_model() -> &'static str {
        DEFAULT_MODEL
    }

    fn model_for_size(size: ModelSize) -> &'static str {
        match size {
            ModelSize::Small => "gemini-2.5-flash-lite",
            ModelSize::Medium => "gemini-2.5-flash",
            ModelSize::Large => "gemini-2.5-pro",
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

    async fn run_resume(&self, session_id: Option<&str>, _last: bool) -> Result<()> {
        let mut cmd = Command::new("gemini");

        if let Some(ref root) = self.root {
            cmd.current_dir(root);
        }

        if let Some(id) = session_id {
            cmd.args(["--resume", id]);
        } else {
            cmd.args(["--resume", "latest"]);
        }

        if self.skip_permissions {
            cmd.args(["--approval-mode", "yolo"]);
        }

        if !self.model.is_empty() && self.model != "auto" {
            cmd.args(["--model", &self.model]);
        }

        for dir in &self.add_dirs {
            cmd.args(["--include-directories", dir]);
        }

        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let status = cmd.status().await?;
        if !status.success() {
            anyhow::bail!("Gemini resume failed with status: {}", status);
        }
        Ok(())
    }

    async fn cleanup(&self) -> Result<()> {
        let base = self.get_base_path();
        let gemini_dir = base.join(".gemini");
        let system_file = gemini_dir.join("system.md");

        if system_file.exists() {
            fs::remove_file(&system_file).await?;
        }

        if gemini_dir.exists()
            && fs::read_dir(&gemini_dir)
                .await?
                .next_entry()
                .await?
                .is_none()
        {
            fs::remove_dir(&gemini_dir).await?;
        }

        Ok(())
    }
}
