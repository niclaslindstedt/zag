use crate::agent::{Agent, ModelSize};
use crate::output::AgentOutput;
use crate::sandbox::SandboxConfig;
use crate::session_log::{
    BackfilledSession, HistoricalLogAdapter, LiveLogAdapter, LiveLogContext, LogCompleteness,
    LogEventKind, LogSourceKind, SessionLogMetadata, SessionLogWriter,
};
use anyhow::Result;
use async_trait::async_trait;
use log::info;
use std::collections::HashSet;
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
    capture_output: bool,
    sandbox: Option<SandboxConfig>,
}

pub struct GeminiLiveLogAdapter {
    ctx: LiveLogContext,
    session_path: Option<std::path::PathBuf>,
    emitted_message_ids: std::collections::HashSet<String>,
}

pub struct GeminiHistoricalLogAdapter;

impl Gemini {
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

    async fn write_system_file(&self) -> Result<()> {
        let base = self.get_base_path();
        log::debug!("Writing Gemini system file to {}", base.display());
        let gemini_dir = base.join(".gemini");
        fs::create_dir_all(&gemini_dir).await?;
        fs::write(gemini_dir.join("system.md"), &self.system_prompt).await?;
        Ok(())
    }

    /// Build the argument list for a run/exec invocation.
    fn build_run_args(&self, interactive: bool, prompt: Option<&str>) -> Vec<String> {
        let mut args = Vec::new();

        if self.skip_permissions {
            args.extend(["--approval-mode", "yolo"].map(String::from));
        }

        if !self.model.is_empty() && self.model != "auto" {
            args.extend(["--model".to_string(), self.model.clone()]);
        }

        for dir in &self.add_dirs {
            args.extend(["--include-directories".to_string(), dir.clone()]);
        }

        if !interactive && let Some(ref format) = self.output_format {
            args.extend(["--output-format".to_string(), format.clone()]);
        }

        if let Some(p) = prompt {
            args.push(p.to_string());
        }

        args
    }

    /// Create a `Command` either directly or wrapped in sandbox.
    fn make_command(&self, agent_args: Vec<String>) -> Command {
        if let Some(ref sb) = self.sandbox {
            let std_cmd = crate::sandbox::build_sandbox_command(sb, agent_args);
            Command::from(std_cmd)
        } else {
            let mut cmd = Command::new("gemini");
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
        if !self.system_prompt.is_empty() {
            log::debug!(
                "Gemini system prompt (written to system.md): {}",
                self.system_prompt
            );
            self.write_system_file().await?;
        }

        let agent_args = self.build_run_args(interactive, prompt);
        log::debug!("Gemini command: gemini {}", agent_args.join(" "));
        if let Some(p) = prompt {
            log::debug!("Gemini user prompt: {}", p);
        }
        let mut cmd = self.make_command(agent_args);

        if !self.system_prompt.is_empty() {
            cmd.env("GEMINI_SYSTEM_MD", "true");
        }

        if interactive {
            cmd.stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
            let status = cmd.status().await?;
            if !status.success() {
                anyhow::bail!("Gemini command failed with status: {}", status);
            }
            Ok(None)
        } else if self.capture_output {
            let text = crate::process::run_captured(&mut cmd, "Gemini").await?;
            log::debug!("Gemini raw response ({} bytes): {}", text.len(), text);
            Ok(Some(AgentOutput::from_text("gemini", &text)))
        } else {
            cmd.stdin(Stdio::inherit()).stdout(Stdio::inherit());
            crate::process::run_with_captured_stderr(&mut cmd).await?;
            Ok(None)
        }
    }
}

#[cfg(test)]
#[path = "gemini_tests.rs"]
mod tests;

impl Default for Gemini {
    fn default() -> Self {
        Self::new()
    }
}

impl GeminiLiveLogAdapter {
    pub fn new(ctx: LiveLogContext) -> Self {
        Self {
            ctx,
            session_path: None,
            emitted_message_ids: HashSet::new(),
        }
    }

    fn discover_session_path(&self) -> Option<std::path::PathBuf> {
        let home = std::env::var_os("HOME").map(std::path::PathBuf::from)?;
        let gemini_tmp = home.join(".gemini/tmp");
        let mut best: Option<(std::time::SystemTime, std::path::PathBuf)> = None;
        let projects = std::fs::read_dir(gemini_tmp).ok()?;
        for project in projects.flatten() {
            let chats = project.path().join("chats");
            let files = std::fs::read_dir(chats).ok()?;
            for file in files.flatten() {
                let path = file.path();
                let metadata = file.metadata().ok()?;
                let modified = metadata.modified().ok()?;
                let started_at = std::time::SystemTime::UNIX_EPOCH
                    + std::time::Duration::from_secs(self.ctx.started_at.timestamp().max(0) as u64);
                if modified < started_at {
                    continue;
                }
                if best
                    .as_ref()
                    .map(|(current, _)| modified > *current)
                    .unwrap_or(true)
                {
                    best = Some((modified, path));
                }
            }
        }
        best.map(|(_, path)| path)
    }
}

#[async_trait]
impl LiveLogAdapter for GeminiLiveLogAdapter {
    async fn poll(&mut self, writer: &SessionLogWriter) -> Result<()> {
        if self.session_path.is_none() {
            self.session_path = self.discover_session_path();
            if let Some(path) = &self.session_path {
                writer.add_source_path(path.to_string_lossy().to_string())?;
            }
        }
        let Some(path) = self.session_path.as_ref() else {
            return Ok(());
        };
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(_) => return Ok(()),
        };
        let json: serde_json::Value = match serde_json::from_str(&content) {
            Ok(json) => json,
            Err(_) => {
                writer.emit(
                    LogSourceKind::ProviderFile,
                    LogEventKind::ParseWarning {
                        message: "Failed to parse Gemini chat file".to_string(),
                        raw: None,
                    },
                )?;
                return Ok(());
            }
        };
        if let Some(session_id) = json.get("sessionId").and_then(|value| value.as_str()) {
            writer.set_provider_session_id(Some(session_id.to_string()))?;
        }
        if let Some(messages) = json.get("messages").and_then(|value| value.as_array()) {
            for message in messages {
                let message_id = message
                    .get("id")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string();
                if message_id.is_empty() || !self.emitted_message_ids.insert(message_id.clone()) {
                    continue;
                }
                match message.get("type").and_then(|value| value.as_str()) {
                    Some("user") => writer.emit(
                        LogSourceKind::ProviderFile,
                        LogEventKind::UserMessage {
                            role: "user".to_string(),
                            content: message
                                .get("content")
                                .and_then(|value| value.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            message_id: Some(message_id.clone()),
                        },
                    )?,
                    Some("gemini") => {
                        writer.emit(
                            LogSourceKind::ProviderFile,
                            LogEventKind::AssistantMessage {
                                content: message
                                    .get("content")
                                    .and_then(|value| value.as_str())
                                    .unwrap_or_default()
                                    .to_string(),
                                message_id: Some(message_id.clone()),
                            },
                        )?;
                        if let Some(thoughts) =
                            message.get("thoughts").and_then(|value| value.as_array())
                        {
                            for thought in thoughts {
                                writer.emit(
                                    LogSourceKind::ProviderFile,
                                    LogEventKind::Reasoning {
                                        content: thought
                                            .get("description")
                                            .and_then(|value| value.as_str())
                                            .unwrap_or_default()
                                            .to_string(),
                                        message_id: Some(message_id.clone()),
                                    },
                                )?;
                            }
                        }
                        writer.emit(
                            LogSourceKind::ProviderFile,
                            LogEventKind::ProviderStatus {
                                message: "Gemini message metadata".to_string(),
                                data: Some(serde_json::json!({
                                    "tokens": message.get("tokens"),
                                    "model": message.get("model"),
                                })),
                            },
                        )?;
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }
}

impl HistoricalLogAdapter for GeminiHistoricalLogAdapter {
    fn backfill(&self, _root: Option<&str>) -> Result<Vec<BackfilledSession>> {
        let mut sessions = Vec::new();
        let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) else {
            return Ok(sessions);
        };
        let gemini_tmp = home.join(".gemini/tmp");
        let projects = match std::fs::read_dir(gemini_tmp) {
            Ok(projects) => projects,
            Err(_) => return Ok(sessions),
        };
        for project in projects.flatten() {
            let chats = project.path().join("chats");
            let files = match std::fs::read_dir(chats) {
                Ok(files) => files,
                Err(_) => continue,
            };
            for file in files.flatten() {
                let path = file.path();
                info!("Scanning Gemini history: {}", path.display());
                let content = match std::fs::read_to_string(&path) {
                    Ok(content) => content,
                    Err(_) => continue,
                };
                let json: serde_json::Value = match serde_json::from_str(&content) {
                    Ok(json) => json,
                    Err(_) => continue,
                };
                let Some(session_id) = json.get("sessionId").and_then(|value| value.as_str())
                else {
                    continue;
                };
                let mut events = Vec::new();
                if let Some(messages) = json.get("messages").and_then(|value| value.as_array()) {
                    for message in messages {
                        let message_id = message
                            .get("id")
                            .and_then(|value| value.as_str())
                            .map(str::to_string);
                        match message.get("type").and_then(|value| value.as_str()) {
                            Some("user") => events.push((
                                LogSourceKind::Backfill,
                                LogEventKind::UserMessage {
                                    role: "user".to_string(),
                                    content: message
                                        .get("content")
                                        .and_then(|value| value.as_str())
                                        .unwrap_or_default()
                                        .to_string(),
                                    message_id: message_id.clone(),
                                },
                            )),
                            Some("gemini") => {
                                events.push((
                                    LogSourceKind::Backfill,
                                    LogEventKind::AssistantMessage {
                                        content: message
                                            .get("content")
                                            .and_then(|value| value.as_str())
                                            .unwrap_or_default()
                                            .to_string(),
                                        message_id: message_id.clone(),
                                    },
                                ));
                                if let Some(thoughts) =
                                    message.get("thoughts").and_then(|value| value.as_array())
                                {
                                    for thought in thoughts {
                                        events.push((
                                            LogSourceKind::Backfill,
                                            LogEventKind::Reasoning {
                                                content: thought
                                                    .get("description")
                                                    .and_then(|value| value.as_str())
                                                    .unwrap_or_default()
                                                    .to_string(),
                                                message_id: message_id.clone(),
                                            },
                                        ));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                sessions.push(BackfilledSession {
                    metadata: SessionLogMetadata {
                        provider: "gemini".to_string(),
                        wrapper_session_id: session_id.to_string(),
                        provider_session_id: Some(session_id.to_string()),
                        workspace_path: None,
                        command: "backfill".to_string(),
                        model: None,
                        resumed: false,
                        backfilled: true,
                    },
                    completeness: LogCompleteness::Full,
                    source_paths: vec![path.to_string_lossy().to_string()],
                    events,
                });
            }
        }
        Ok(sessions)
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

    fn set_capture_output(&mut self, capture: bool) {
        self.capture_output = capture;
    }

    fn set_sandbox(&mut self, config: SandboxConfig) {
        self.sandbox = Some(config);
    }

    fn as_any_ref(&self) -> &dyn std::any::Any {
        self
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

    async fn run_resume(&self, session_id: Option<&str>, _last: bool) -> Result<()> {
        let mut args = Vec::new();

        if let Some(id) = session_id {
            args.extend(["--resume".to_string(), id.to_string()]);
        } else {
            args.extend(["--resume".to_string(), "latest".to_string()]);
        }

        if self.skip_permissions {
            args.extend(["--approval-mode", "yolo"].map(String::from));
        }

        if !self.model.is_empty() && self.model != "auto" {
            args.extend(["--model".to_string(), self.model.clone()]);
        }

        for dir in &self.add_dirs {
            args.extend(["--include-directories".to_string(), dir.clone()]);
        }

        let mut cmd = self.make_command(args);

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
        log::debug!("Cleaning up Gemini agent resources");
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
