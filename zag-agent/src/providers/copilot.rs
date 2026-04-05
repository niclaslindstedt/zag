// provider-updated: 2026-04-05
use crate::agent::{Agent, ModelSize};
use crate::output::AgentOutput;
use crate::sandbox::SandboxConfig;
use crate::session_log::{
    BackfilledSession, HistoricalLogAdapter, LiveLogAdapter, LiveLogContext, LogCompleteness,
    LogEventKind, LogSourceKind, SessionLogMetadata, SessionLogWriter, ToolKind,
};
use anyhow::{Context, Result};

/// Classify a Copilot tool name into a normalized ToolKind.
fn tool_kind_from_name(name: &str) -> ToolKind {
    match name {
        "bash" | "shell" => ToolKind::Shell,
        "view" | "read" | "cat" => ToolKind::FileRead,
        "write" => ToolKind::FileWrite,
        "edit" | "insert" | "replace" => ToolKind::FileEdit,
        "grep" | "glob" | "find" | "search" => ToolKind::Search,
        _ => ToolKind::Other,
    }
}
use async_trait::async_trait;
use log::info;
use std::collections::HashSet;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;

/// Return the Copilot session-state directory: `~/.copilot/session-state/`.
pub fn session_state_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".copilot/session-state")
}

pub const DEFAULT_MODEL: &str = "claude-sonnet-4.6";

pub const AVAILABLE_MODELS: &[&str] = &[
    "claude-sonnet-4.6",
    "claude-haiku-4.5",
    "claude-opus-4.6",
    "claude-sonnet-4.5",
    "claude-opus-4.5",
    "gpt-5.4",
    "gpt-5.4-mini",
    "gpt-5.3-codex",
    "gpt-5.2-codex",
    "gpt-5.2",
    "gpt-5.1-codex-max",
    "gpt-5.1-codex",
    "gpt-5.1",
    "gpt-5",
    "gpt-5.1-codex-mini",
    "gpt-5-mini",
    "gpt-4.1",
    "gemini-3.1-pro-preview",
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
    max_turns: Option<u32>,
    env_vars: Vec<(String, String)>,
}

pub struct CopilotLiveLogAdapter {
    ctx: LiveLogContext,
    session_path: Option<PathBuf>,
    offset: u64,
    seen_event_ids: HashSet<String>,
}

pub struct CopilotHistoricalLogAdapter;

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
            max_turns: None,
            env_vars: Vec::new(),
        }
    }

    fn get_base_path(&self) -> &Path {
        self.root.as_ref().map(Path::new).unwrap_or(Path::new("."))
    }

    async fn write_instructions_file(&self) -> Result<()> {
        let base = self.get_base_path();
        log::debug!("Writing Copilot instructions file to {}", base.display());
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

        // In non-interactive mode, --allow-all is required
        if !interactive || self.skip_permissions {
            args.push("--allow-all".to_string());
        }

        if !self.model.is_empty() {
            args.extend(["--model".to_string(), self.model.clone()]);
        }

        for dir in &self.add_dirs {
            args.extend(["--add-dir".to_string(), dir.clone()]);
        }

        if let Some(turns) = self.max_turns {
            args.extend(["--max-turns".to_string(), turns.to_string()]);
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
            for (key, value) in &self.env_vars {
                cmd.env(key, value);
            }
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
            log::debug!(
                "Copilot system prompt (written to instructions): {}",
                self.system_prompt
            );
            self.write_instructions_file().await?;
        }

        let agent_args = self.build_run_args(interactive, prompt);
        log::debug!("Copilot command: copilot {}", agent_args.join(" "));
        if let Some(p) = prompt {
            log::debug!("Copilot user prompt: {}", p);
        }
        let mut cmd = self.make_command(agent_args);

        if interactive {
            cmd.stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
            let status = cmd
                .status()
                .await
                .context("Failed to execute 'copilot' CLI. Is it installed and in PATH?")?;
            if !status.success() {
                anyhow::bail!("Copilot command failed with status: {}", status);
            }
            Ok(None)
        } else if self.capture_output {
            let text = crate::process::run_captured(&mut cmd, "Copilot").await?;
            log::debug!("Copilot raw response ({} bytes): {}", text.len(), text);
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

impl CopilotLiveLogAdapter {
    pub fn new(ctx: LiveLogContext) -> Self {
        Self {
            ctx,
            session_path: None,
            offset: 0,
            seen_event_ids: HashSet::new(),
        }
    }

    fn discover_session_path(&self) -> Option<PathBuf> {
        let base = copilot_session_state_dir();
        if let Some(session_id) = &self.ctx.provider_session_id {
            let candidate = base.join(session_id).join("events.jsonl");
            if candidate.exists() {
                return Some(candidate);
            }
        }

        let started_at = system_time_from_utc(self.ctx.started_at);
        let workspace = self
            .ctx
            .workspace_path
            .as_deref()
            .or(self.ctx.root.as_deref());
        let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
        let entries = std::fs::read_dir(base).ok()?;
        for entry in entries.flatten() {
            let path = entry.path().join("events.jsonl");
            if !path.exists() {
                continue;
            }
            let modified = entry
                .metadata()
                .ok()
                .and_then(|metadata| metadata.modified().ok())
                .or_else(|| {
                    std::fs::metadata(&path)
                        .ok()
                        .and_then(|metadata| metadata.modified().ok())
                })?;
            if modified < started_at {
                continue;
            }
            if let Some(workspace) = workspace
                && !copilot_session_matches_workspace(&entry.path(), workspace)
            {
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
        best.map(|(_, path)| path)
    }
}

#[async_trait]
impl LiveLogAdapter for CopilotLiveLogAdapter {
    async fn poll(&mut self, writer: &SessionLogWriter) -> Result<()> {
        if self.session_path.is_none() {
            self.session_path = self.discover_session_path();
            if let Some(path) = &self.session_path {
                writer.add_source_path(path.to_string_lossy().to_string())?;
                let metadata_path = path.with_file_name("vscode.metadata.json");
                if metadata_path.exists() {
                    writer.add_source_path(metadata_path.to_string_lossy().to_string())?;
                }
                let workspace_path = path.with_file_name("workspace.yaml");
                if workspace_path.exists() {
                    writer.add_source_path(workspace_path.to_string_lossy().to_string())?;
                }
            }
        }

        let Some(path) = self.session_path.as_ref() else {
            return Ok(());
        };

        let mut file = std::fs::File::open(path)
            .with_context(|| format!("Failed to open {}", path.display()))?;
        file.seek(SeekFrom::Start(self.offset))?;
        let mut reader = BufReader::new(file);
        let mut line = String::new();

        while reader.read_line(&mut line)? > 0 {
            self.offset += line.len() as u64;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                line.clear();
                continue;
            }
            let Some(parsed) = parse_copilot_event_line(trimmed, &mut self.seen_event_ids) else {
                line.clear();
                continue;
            };
            if parsed.parse_failed {
                writer.emit(
                    LogSourceKind::ProviderFile,
                    LogEventKind::ParseWarning {
                        message: "Failed to parse Copilot event line".to_string(),
                        raw: Some(trimmed.to_string()),
                    },
                )?;
                line.clear();
                continue;
            }
            if let Some(session_id) = parsed.provider_session_id {
                writer.set_provider_session_id(Some(session_id))?;
            }
            for event in parsed.events {
                writer.emit(LogSourceKind::ProviderFile, event)?;
            }
            line.clear();
        }

        Ok(())
    }
}

impl HistoricalLogAdapter for CopilotHistoricalLogAdapter {
    fn backfill(&self, _root: Option<&str>) -> Result<Vec<BackfilledSession>> {
        let base = copilot_session_state_dir();
        let entries = match std::fs::read_dir(&base) {
            Ok(entries) => entries,
            Err(_) => return Ok(Vec::new()),
        };
        let mut sessions = Vec::new();
        for entry in entries.flatten() {
            let session_dir = entry.path();
            if !session_dir.is_dir() {
                continue;
            }
            let events_path = session_dir.join("events.jsonl");
            if !events_path.exists() {
                continue;
            }
            info!("Scanning Copilot history: {}", events_path.display());
            let file = std::fs::File::open(&events_path)
                .with_context(|| format!("Failed to open {}", events_path.display()))?;
            let reader = BufReader::new(file);
            let mut seen_event_ids = HashSet::new();
            let mut events = Vec::new();
            let mut provider_session_id = None;
            let mut model = None;
            let mut workspace_path = read_copilot_workspace_path(&session_dir);

            for line in reader.lines() {
                let line = line?;
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let Some(parsed) = parse_copilot_event_line(trimmed, &mut seen_event_ids) else {
                    continue;
                };
                if parsed.parse_failed {
                    events.push((
                        LogSourceKind::Backfill,
                        LogEventKind::ParseWarning {
                            message: "Failed to parse Copilot event line".to_string(),
                            raw: Some(trimmed.to_string()),
                        },
                    ));
                    continue;
                }
                if provider_session_id.is_none() {
                    provider_session_id = parsed.provider_session_id;
                }
                if model.is_none() {
                    model = parsed.model;
                }
                if workspace_path.is_none() {
                    workspace_path = parsed.workspace_path;
                }
                for event in parsed.events {
                    events.push((LogSourceKind::Backfill, event));
                }
            }

            let session_id = provider_session_id
                .unwrap_or_else(|| entry.file_name().to_string_lossy().to_string());
            let mut source_paths = vec![events_path.to_string_lossy().to_string()];
            let metadata_path = session_dir.join("vscode.metadata.json");
            if metadata_path.exists() {
                source_paths.push(metadata_path.to_string_lossy().to_string());
            }
            let workspace_yaml = session_dir.join("workspace.yaml");
            if workspace_yaml.exists() {
                source_paths.push(workspace_yaml.to_string_lossy().to_string());
            }
            sessions.push(BackfilledSession {
                metadata: SessionLogMetadata {
                    provider: "copilot".to_string(),
                    wrapper_session_id: session_id.clone(),
                    provider_session_id: Some(session_id),
                    workspace_path,
                    command: "backfill".to_string(),
                    model,
                    resumed: false,
                    backfilled: true,
                },
                completeness: LogCompleteness::Full,
                source_paths,
                events,
            });
        }
        Ok(sessions)
    }
}

pub(crate) struct ParsedCopilotEvent {
    pub(crate) provider_session_id: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) workspace_path: Option<String>,
    pub(crate) events: Vec<LogEventKind>,
    pub(crate) parse_failed: bool,
}

pub(crate) fn parse_copilot_event_line(
    line: &str,
    seen_event_ids: &mut HashSet<String>,
) -> Option<ParsedCopilotEvent> {
    let value: serde_json::Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(_) => {
            return Some(ParsedCopilotEvent {
                provider_session_id: None,
                model: None,
                workspace_path: None,
                events: Vec::new(),
                parse_failed: true,
            });
        }
    };

    let event_id = value
        .get("id")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    if !event_id.is_empty() && !seen_event_ids.insert(event_id.to_string()) {
        return None;
    }

    let event_type = value
        .get("type")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let data = value
        .get("data")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let provider_session_id = value
        .get("data")
        .and_then(|value| value.get("sessionId"))
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let model = value
        .get("data")
        .and_then(|value| value.get("selectedModel"))
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let workspace_path = value
        .get("data")
        .and_then(|value| value.get("context"))
        .and_then(|value| value.get("cwd").or_else(|| value.get("gitRoot")))
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let mut events = Vec::new();

    match event_type {
        "session.start" => events.push(LogEventKind::ProviderStatus {
            message: "Copilot session started".to_string(),
            data: Some(data),
        }),
        "session.model_change" => events.push(LogEventKind::ProviderStatus {
            message: "Copilot model changed".to_string(),
            data: Some(data),
        }),
        "session.info" => events.push(LogEventKind::ProviderStatus {
            message: data
                .get("message")
                .and_then(|value| value.as_str())
                .unwrap_or("Copilot session info")
                .to_string(),
            data: Some(data),
        }),
        "session.truncation" => events.push(LogEventKind::ProviderStatus {
            message: "Copilot session truncation".to_string(),
            data: Some(data),
        }),
        "user.message" => events.push(LogEventKind::UserMessage {
            role: "user".to_string(),
            content: data
                .get("content")
                .or_else(|| data.get("transformedContent"))
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string(),
            message_id: value
                .get("id")
                .and_then(|value| value.as_str())
                .map(str::to_string),
        }),
        "assistant.turn_start" => events.push(LogEventKind::ProviderStatus {
            message: "Copilot assistant turn started".to_string(),
            data: Some(data),
        }),
        "assistant.turn_end" => events.push(LogEventKind::ProviderStatus {
            message: "Copilot assistant turn ended".to_string(),
            data: Some(data),
        }),
        "assistant.message" => {
            let message_id = data
                .get("messageId")
                .and_then(|value| value.as_str())
                .map(str::to_string);
            let content = data
                .get("content")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string();
            if !content.is_empty() {
                events.push(LogEventKind::AssistantMessage {
                    content,
                    message_id: message_id.clone(),
                });
            }
            if let Some(tool_requests) = data.get("toolRequests").and_then(|value| value.as_array())
            {
                for request in tool_requests {
                    let name = request
                        .get("name")
                        .and_then(|value| value.as_str())
                        .unwrap_or_default();
                    events.push(LogEventKind::ToolCall {
                        tool_kind: Some(tool_kind_from_name(name)),
                        tool_name: name.to_string(),
                        tool_id: request
                            .get("toolCallId")
                            .and_then(|value| value.as_str())
                            .map(str::to_string),
                        input: request.get("arguments").cloned(),
                    });
                }
            }
        }
        "assistant.reasoning" => {
            let content = data
                .get("content")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string();
            if !content.is_empty() {
                events.push(LogEventKind::Reasoning {
                    content,
                    message_id: data
                        .get("reasoningId")
                        .and_then(|value| value.as_str())
                        .map(str::to_string),
                });
            }
        }
        "tool.execution_start" => {
            let name = data
                .get("toolName")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            events.push(LogEventKind::ToolCall {
                tool_kind: Some(tool_kind_from_name(name)),
                tool_name: name.to_string(),
                tool_id: data
                    .get("toolCallId")
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                input: data.get("arguments").cloned(),
            });
        }
        "tool.execution_complete" => {
            let name = data.get("toolName").and_then(|value| value.as_str());
            events.push(LogEventKind::ToolResult {
                tool_kind: name.map(tool_kind_from_name),
                tool_name: name.map(str::to_string),
                tool_id: data
                    .get("toolCallId")
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                success: data.get("success").and_then(|value| value.as_bool()),
                output: data
                    .get("result")
                    .and_then(|value| value.get("content"))
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                error: data
                    .get("result")
                    .and_then(|value| value.get("error"))
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                data: Some(data),
            });
        }
        _ => events.push(LogEventKind::ProviderStatus {
            message: format!("Copilot event: {event_type}"),
            data: Some(data),
        }),
    }

    Some(ParsedCopilotEvent {
        provider_session_id,
        model,
        workspace_path,
        events,
        parse_failed: false,
    })
}

fn copilot_session_state_dir() -> PathBuf {
    session_state_dir()
}

fn read_copilot_workspace_path(session_dir: &Path) -> Option<String> {
    let metadata_path = session_dir.join("vscode.metadata.json");
    if let Ok(content) = std::fs::read_to_string(&metadata_path)
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(&content)
    {
        if let Some(path) = value
            .get("cwd")
            .or_else(|| value.get("workspacePath"))
            .or_else(|| value.get("gitRoot"))
            .and_then(|value| value.as_str())
        {
            return Some(path.to_string());
        }
    }
    let workspace_yaml = session_dir.join("workspace.yaml");
    if let Ok(content) = std::fs::read_to_string(workspace_yaml) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed
                .strip_prefix("cwd:")
                .or_else(|| trimmed.strip_prefix("workspace:"))
                .or_else(|| trimmed.strip_prefix("path:"))
            {
                return Some(rest.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

fn copilot_session_matches_workspace(session_dir: &Path, workspace: &str) -> bool {
    if let Some(candidate) = read_copilot_workspace_path(session_dir) {
        return candidate == workspace;
    }

    let events_path = session_dir.join("events.jsonl");
    let file = match std::fs::File::open(events_path) {
        Ok(file) => file,
        Err(_) => return false,
    };
    let reader = BufReader::new(file);
    for line in reader.lines().map_while(Result::ok).take(8) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        let Some(data) = value.get("data") else {
            continue;
        };
        let candidate = data
            .get("context")
            .and_then(|context| context.get("cwd").or_else(|| context.get("gitRoot")))
            .and_then(|value| value.as_str());
        if candidate == Some(workspace) {
            return true;
        }
    }
    false
}

fn system_time_from_utc(value: chrono::DateTime<chrono::Utc>) -> std::time::SystemTime {
    std::time::SystemTime::UNIX_EPOCH
        + std::time::Duration::from_secs(value.timestamp().max(0) as u64)
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
            ModelSize::Medium => "claude-sonnet-4.6",
            ModelSize::Large => "claude-opus-4.6",
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

    fn set_env_vars(&mut self, vars: Vec<(String, String)>) {
        self.env_vars = vars;
    }

    fn set_capture_output(&mut self, capture: bool) {
        self.capture_output = capture;
    }

    fn set_sandbox(&mut self, config: SandboxConfig) {
        self.sandbox = Some(config);
    }

    fn set_max_turns(&mut self, turns: u32) {
        self.max_turns = Some(turns);
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

    async fn run_resume(&self, session_id: Option<&str>, last: bool) -> Result<()> {
        let mut args = if let Some(session_id) = session_id {
            vec!["--resume".to_string(), session_id.to_string()]
        } else if last {
            vec!["--continue".to_string()]
        } else {
            vec!["--resume".to_string()]
        };

        if self.skip_permissions {
            args.push("--allow-all".to_string());
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

        let status = cmd
            .status()
            .await
            .context("Failed to execute 'copilot' CLI. Is it installed and in PATH?")?;
        if !status.success() {
            anyhow::bail!("Copilot resume failed with status: {}", status);
        }
        Ok(())
    }

    async fn cleanup(&self) -> Result<()> {
        log::debug!("Cleaning up Copilot agent resources");
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
