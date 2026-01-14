use crate::agent::Agent;
use crate::process::wait_with_pid_tracking;
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;

pub const DEFAULT_MODEL: &str = "auto";

pub const AVAILABLE_MODELS: &[(&str, &str)] = &[
    ("auto", "Let the system choose the best model for your task"),
    (
        "gemini-2.5-pro",
        "For complex tasks that require deep reasoning and creativity",
    ),
    (
        "gemini-2.5-flash",
        "For tasks that need a balance of speed and reasoning",
    ),
    (
        "gemini-2.5-flash-lite",
        "For simple tasks that need to be done quickly",
    ),
];

pub struct Gemini {
    system_prompt: String,
    model: String,
    root: Option<String>,
    skip_permissions: bool,
}

impl Gemini {
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

    async fn write_system_file(&self) -> Result<()> {
        let base = self.get_base_path();
        let gemini_dir = base.join(".gemini");
        fs::create_dir_all(&gemini_dir).await?;
        fs::write(gemini_dir.join("system.md"), &self.system_prompt).await?;
        Ok(())
    }

    async fn execute(&self, interactive: bool, prompt: &str) -> Result<()> {
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

        if interactive {
            cmd.arg("--prompt-interactive");
        } else {
            cmd.args(["--output-format", "json"]);
        }

        cmd.arg(prompt);

        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let child = cmd.spawn()?;
        wait_with_pid_tracking(child).await
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
