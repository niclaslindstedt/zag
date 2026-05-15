// provider-updated: 2026-04-05
use crate::agent::{Agent, ModelSize};
use crate::output::AgentOutput;
use crate::providers::common::CommonAgentState;
use crate::session_log::{
    BackfilledSession, HistoricalLogAdapter, LiveLogAdapter, LiveLogContext, LogCompleteness,
    LogEventKind, LogSourceKind, SessionLogMetadata, SessionLogWriter,
};
use anyhow::Result;
use async_trait::async_trait;
use log::info;
use std::collections::HashSet;
use tokio::fs;
use tokio::process::Command;

/// Return the Gemini tmp directory: `~/.gemini/tmp/`.
pub fn tmp_dir() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join(".gemini/tmp"))
}

pub const DEFAULT_MODEL: &str = "auto";

pub const AVAILABLE_MODELS: &[&str] = &[
    "auto",
    "gemini-3.1-pro-preview",
    "gemini-3.1-flash-lite-preview",
    "gemini-3-pro-preview",
    "gemini-3-flash-preview",
    "gemini-2.5-pro",
    "gemini-2.5-flash",
    "gemini-2.5-flash-lite",
];

pub struct Gemini {
    pub common: CommonAgentState,
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
            common: CommonAgentState::new(DEFAULT_MODEL),
        }
    }

    async fn write_system_file(&self) -> Result<()> {
        let base = self.common.get_base_path();
        log::debug!("Writing Gemini system file to {}", base.display());
        let gemini_dir = base.join(".gemini");
        fs::create_dir_all(&gemini_dir).await?;
        fs::write(gemini_dir.join("system.md"), &self.common.system_prompt).await?;
        Ok(())
    }

    /// Build the argument list for a run/exec invocation.
    fn build_run_args(&self, interactive: bool, prompt: Option<&str>) -> Vec<String> {
        let mut args = Vec::new();

        if self.common.skip_permissions {
            args.extend(["--approval-mode", "yolo"].map(String::from));
        }

        if !self.common.model.is_empty() && self.common.model != "auto" {
            args.extend(["--model".to_string(), self.common.model.clone()]);
        }

        for dir in &self.common.add_dirs {
            args.extend(["--include-directories".to_string(), dir.clone()]);
        }

        if !interactive && let Some(ref format) = self.common.output_format {
            args.extend(["--output-format".to_string(), format.clone()]);
        }

        // Note: Gemini CLI does not support --max-turns as a CLI flag.
        // Max turns is configured via settings.json (maxSessionTurns).
        // The value is stored but not passed as an argument.

        if let Some(p) = prompt {
            // End option parsing so prompts that start with `-` / `--`
            // aren't misread as flags by the gemini CLI.
            args.push("--".to_string());
            args.push(p.to_string());
        }

        args
    }

    /// Create a `Command` either directly or wrapped in sandbox.
    fn make_command(&self, agent_args: Vec<String>) -> Command {
        self.common.make_command("gemini", agent_args)
    }

    /// Build args for `--resume <id> -- <prompt>` headless invocation.
    ///
    /// Mirrors `build_run_args(false, Some(prompt))` plus `--resume <id>`.
    /// The trailing `--` ends option parsing so prompts beginning with `-`
    /// aren't misread as flags by the gemini CLI. Used by
    /// [`Agent::run_resume_with_prompt`] for auto-resume after a detected
    /// usage limit.
    fn build_resume_args(&self, session_id: &str, prompt: &str) -> Vec<String> {
        let mut args = Vec::new();

        if self.common.skip_permissions {
            args.extend(["--approval-mode", "yolo"].map(String::from));
        }

        if !self.common.model.is_empty() && self.common.model != "auto" {
            args.extend(["--model".to_string(), self.common.model.clone()]);
        }

        for dir in &self.common.add_dirs {
            args.extend(["--include-directories".to_string(), dir.clone()]);
        }

        if let Some(ref format) = self.common.output_format {
            args.extend(["--output-format".to_string(), format.clone()]);
        }

        args.extend(["--resume".to_string(), session_id.to_string()]);
        args.push("--".to_string());
        args.push(prompt.to_string());
        args
    }

    async fn execute(
        &self,
        interactive: bool,
        prompt: Option<&str>,
    ) -> Result<Option<AgentOutput>> {
        if !self.common.system_prompt.is_empty() {
            log::debug!(
                "Gemini system prompt (written to system.md): {}",
                self.common.system_prompt
            );
            self.write_system_file().await?;
        }

        let agent_args = self.build_run_args(interactive, prompt);
        log::debug!("Gemini command: gemini {}", agent_args.join(" "));
        if let Some(p) = prompt {
            log::debug!("Gemini user prompt: {p}");
        }
        let mut cmd = self.make_command(agent_args);

        if !self.common.system_prompt.is_empty() {
            cmd.env("GEMINI_SYSTEM_MD", "true");
        }

        if interactive {
            self.common
                .run_interactive_dispatch(&mut cmd, "Gemini")
                .await?;
            Ok(None)
        } else {
            self.common
                .run_non_interactive_simple(&mut cmd, "Gemini")
                .await
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
        let gemini_tmp = tmp_dir()?;
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
        // Scan the whole chat blob for a Gemini 429 / RESOURCE_EXHAUSTED
        // envelope. The canonical signal lives on stderr, but some Gemini
        // versions also leak it into the chat file as a system message, and
        // user-supplied `extra_patterns` may match arbitrary content. Dedup
        // by the matched substring so we don't re-emit on every poll cycle.
        {
            let cfg = crate::usage_limits::UsageLimitConfig::default();
            if let Some(hit) = crate::providers::gemini_usage_limits::detect_text(&content, &cfg) {
                let key = format!("usage_limit:{}", hit.raw);
                if self.emitted_message_ids.insert(key) {
                    writer.emit(
                        LogSourceKind::ProviderFile,
                        crate::usage_limits::to_log_event_hit(hit),
                    )?;
                }
            }
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
        let Some(gemini_tmp) = tmp_dir() else {
            return Ok(sessions);
        };
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
            ModelSize::Small => "gemini-3.1-flash-lite-preview",
            ModelSize::Medium => "gemini-2.5-flash",
            ModelSize::Large => "gemini-3.1-pro-preview",
        }
    }

    fn available_models() -> &'static [&'static str] {
        AVAILABLE_MODELS
    }

    crate::providers::common::impl_common_agent_setters!();

    fn set_skip_permissions(&mut self, skip: bool) {
        self.common.skip_permissions = skip;
    }

    crate::providers::common::impl_as_any!();

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

        if self.common.skip_permissions {
            args.extend(["--approval-mode", "yolo"].map(String::from));
        }

        if !self.common.model.is_empty() && self.common.model != "auto" {
            args.extend(["--model".to_string(), self.common.model.clone()]);
        }

        for dir in &self.common.add_dirs {
            args.extend(["--include-directories".to_string(), dir.clone()]);
        }

        let mut cmd = self.make_command(args);
        self.common
            .run_interactive_dispatch(&mut cmd, "Gemini")
            .await
    }

    async fn run_resume_with_prompt(
        &self,
        session_id: &str,
        prompt: &str,
    ) -> Result<Option<AgentOutput>> {
        log::debug!("Gemini resume with prompt: session={session_id}, prompt={prompt}");

        if !self.common.system_prompt.is_empty() {
            self.write_system_file().await?;
        }

        let args = self.build_resume_args(session_id, prompt);
        let mut cmd = self.make_command(args);

        if !self.common.system_prompt.is_empty() {
            cmd.env("GEMINI_SYSTEM_MD", "true");
        }

        self.common
            .run_non_interactive_simple(&mut cmd, "Gemini")
            .await
    }

    /// Cheap startup probe that runs `gemini --version` with a short
    /// timeout. This catches common "binary exists but can't launch"
    /// failures (broken node install, missing dynamic deps, etc.) without
    /// consuming any API quota. Auth failures only surface during real
    /// invocations, so they are picked up later by the run() path.
    async fn probe(&self) -> Result<()> {
        use anyhow::Context;
        use std::time::Duration;
        let probe = async {
            let out = Command::new("gemini")
                .arg("--version")
                .output()
                .await
                .context("failed to launch 'gemini --version'")?;
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                anyhow::bail!(
                    "'gemini --version' exited with {}: {}",
                    out.status,
                    stderr.trim()
                );
            }
            Ok(())
        };
        match tokio::time::timeout(Duration::from_secs(5), probe).await {
            Ok(res) => res,
            Err(_) => anyhow::bail!("'gemini --version' timed out after 5s"),
        }
    }

    async fn cleanup(&self) -> Result<()> {
        log::debug!("Cleaning up Gemini agent resources");
        let base = self.common.get_base_path();
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
