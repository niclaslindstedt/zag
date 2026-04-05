// provider-updated: 2026-04-05
use crate::agent::{Agent, ModelSize};
use crate::output::AgentOutput;
use crate::sandbox::SandboxConfig;
use crate::session_log::{
    BackfilledSession, HistoricalLogAdapter, LiveLogAdapter, LiveLogContext, LogCompleteness,
    LogEventKind, LogSourceKind, SessionLogMetadata, SessionLogWriter, ToolKind,
};
use anyhow::{Context, Result};
use log::debug;

/// Classify a Codex tool name into a normalized ToolKind.
fn tool_kind_from_name(name: &str) -> ToolKind {
    match name {
        "shell" | "bash" => ToolKind::Shell,
        "read_file" | "view" => ToolKind::FileRead,
        "write_file" => ToolKind::FileWrite,
        "apply_patch" | "edit_file" => ToolKind::FileEdit,
        "grep" | "find" | "search" => ToolKind::Search,
        _ => ToolKind::Other,
    }
}
use async_trait::async_trait;
use log::info;
use std::io::BufRead;
use std::path::Path;
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;

/// Return the Codex history file path: `~/.codex/history.jsonl`.
pub fn history_path() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".codex/history.jsonl")
}

/// Return the Codex TUI log path: `~/.codex/log/codex-tui.log`.
pub fn tui_log_path() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".codex/log/codex-tui.log")
}

pub const DEFAULT_MODEL: &str = "gpt-5.4";

pub const AVAILABLE_MODELS: &[&str] = &[
    "gpt-5.4",
    "gpt-5.4-mini",
    "gpt-5.3-codex-spark",
    "gpt-5.3-codex",
    "gpt-5-codex",
    "gpt-5.2-codex",
    "gpt-5.2",
    "o4-mini",
    "gpt-5.1-codex-max",
    "gpt-5.1-codex-mini",
];

pub struct Codex {
    system_prompt: String,
    model: String,
    root: Option<String>,
    skip_permissions: bool,
    output_format: Option<String>,
    add_dirs: Vec<String>,
    capture_output: bool,
    sandbox: Option<SandboxConfig>,
    max_turns: Option<u32>,
    ephemeral: bool,
    output_schema: Option<String>,
}

pub struct CodexLiveLogAdapter {
    _ctx: LiveLogContext,
    tui_offset: u64,
    history_offset: u64,
    thread_id: Option<String>,
    pending_history: Vec<(String, String)>,
}

pub struct CodexHistoricalLogAdapter;

impl Codex {
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
            ephemeral: false,
            output_schema: None,
        }
    }

    pub fn set_ephemeral(&mut self, ephemeral: bool) {
        self.ephemeral = ephemeral;
    }

    /// Set a JSON Schema file path for structured output validation.
    ///
    /// The Codex CLI's `--output-schema` flag accepts a path to a JSON Schema
    /// file that constrains the model's response shape.
    pub fn set_output_schema(&mut self, schema: Option<String>) {
        self.output_schema = schema;
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
            cmd.arg("--full-auto");
        }

        cmd.stdin(Stdio::inherit()).stdout(Stdio::inherit());

        crate::process::run_with_captured_stderr(&mut cmd).await?;
        Ok(())
    }

    /// Parse Codex NDJSON output to extract thread_id and agent message text.
    ///
    /// Codex's `--json` flag outputs streaming JSON events (NDJSON format).
    /// The actual agent response is inside `item.completed` events where
    /// `item.type == "agent_message"`. The thread_id is in the `thread.started` event.
    fn parse_ndjson_output(raw: &str) -> (Option<String>, Option<String>) {
        let mut thread_id = None;
        let mut agent_text = String::new();

        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Ok(event) = serde_json::from_str::<serde_json::Value>(line) {
                match event.get("type").and_then(|t| t.as_str()) {
                    Some("thread.started") => {
                        thread_id = event
                            .get("thread_id")
                            .and_then(|t| t.as_str())
                            .map(String::from);
                    }
                    Some("item.completed") => {
                        if let Some(item) = event.get("item")
                            && item.get("type").and_then(|t| t.as_str()) == Some("agent_message")
                            && let Some(text) = item.get("text").and_then(|t| t.as_str())
                        {
                            if !agent_text.is_empty() {
                                agent_text.push('\n');
                            }
                            agent_text.push_str(text);
                        }
                    }
                    Some("turn.failed") => {
                        let error_msg = event
                            .get("error")
                            .and_then(|e| e.as_str())
                            .unwrap_or("unknown error");
                        if !agent_text.is_empty() {
                            agent_text.push('\n');
                        }
                        agent_text.push_str("[turn failed: ");
                        agent_text.push_str(error_msg);
                        agent_text.push(']');
                    }
                    _ => {}
                }
            }
        }

        let text = if agent_text.is_empty() {
            None
        } else {
            Some(agent_text)
        };
        (thread_id, text)
    }

    /// Build an AgentOutput from raw codex output, parsing NDJSON if output_format is "json".
    fn build_output(&self, raw: &str) -> AgentOutput {
        if self.output_format.as_deref() == Some("json") {
            let (thread_id, agent_text) = Self::parse_ndjson_output(raw);
            let text = agent_text.unwrap_or_else(|| raw.to_string());
            let mut output = AgentOutput::from_text("codex", &text);
            if let Some(tid) = thread_id {
                debug!("Codex thread_id for retries: {}", tid);
                output.session_id = tid;
            }
            output
        } else {
            AgentOutput::from_text("codex", raw)
        }
    }

    /// Build the argument list for a run/exec invocation.
    fn build_run_args(&self, interactive: bool, prompt: Option<&str>) -> Vec<String> {
        let mut args = Vec::new();
        let in_sandbox = self.sandbox.is_some();

        if !interactive {
            args.extend(["exec", "--skip-git-repo-check"].map(String::from));
            if let Some(ref format) = self.output_format
                && format == "json"
            {
                args.push("--json".to_string());
            }
            if self.ephemeral {
                args.push("--ephemeral".to_string());
            }
        }

        // Skip --cd in sandbox (workspace handles root)
        if !in_sandbox && let Some(ref root) = self.root {
            args.extend(["--cd".to_string(), root.clone()]);
        }

        args.extend(["--model".to_string(), self.model.clone()]);

        for dir in &self.add_dirs {
            args.extend(["--add-dir".to_string(), dir.clone()]);
        }

        if self.skip_permissions {
            args.push("--full-auto".to_string());
        }

        if let Some(turns) = self.max_turns {
            args.extend(["--max-turns".to_string(), turns.to_string()]);
        }

        if !interactive && let Some(ref schema) = self.output_schema {
            args.extend(["--output-schema".to_string(), schema.clone()]);
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
            let mut cmd = Command::new("codex");
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
                "Codex system prompt (written to AGENTS.md): {}",
                self.system_prompt
            );
            self.write_agents_file().await?;
        }

        let agent_args = self.build_run_args(interactive, prompt);
        log::debug!("Codex command: codex {}", agent_args.join(" "));
        if let Some(p) = prompt {
            log::debug!("Codex user prompt: {}", p);
        }
        let mut cmd = self.make_command(agent_args);

        if interactive {
            cmd.stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
            let status = cmd
                .status()
                .await
                .context("Failed to execute 'codex' CLI. Is it installed and in PATH?")?;
            if !status.success() {
                anyhow::bail!("Codex command failed with status: {}", status);
            }
            Ok(None)
        } else if self.capture_output {
            let raw = crate::process::run_captured(&mut cmd, "Codex").await?;
            log::debug!("Codex raw response ({} bytes): {}", raw.len(), raw);
            Ok(Some(self.build_output(&raw)))
        } else {
            cmd.stdin(Stdio::inherit()).stdout(Stdio::inherit());
            crate::process::run_with_captured_stderr(&mut cmd).await?;
            Ok(None)
        }
    }
}

#[cfg(test)]
#[path = "codex_tests.rs"]
mod tests;

impl Default for Codex {
    fn default() -> Self {
        Self::new()
    }
}

impl CodexLiveLogAdapter {
    pub fn new(ctx: LiveLogContext) -> Self {
        Self {
            _ctx: ctx,
            tui_offset: file_len(&codex_tui_log_path()).unwrap_or(0),
            history_offset: file_len(&codex_history_path()).unwrap_or(0),
            thread_id: None,
            pending_history: Vec::new(),
        }
    }
}

#[async_trait]
impl LiveLogAdapter for CodexLiveLogAdapter {
    async fn poll(&mut self, writer: &SessionLogWriter) -> Result<()> {
        self.poll_tui(writer)?;
        self.poll_history(writer)?;
        Ok(())
    }
}

impl CodexLiveLogAdapter {
    fn poll_tui(&mut self, writer: &SessionLogWriter) -> Result<()> {
        let path = codex_tui_log_path();
        if !path.exists() {
            return Ok(());
        }
        let mut reader = open_reader_from_offset(&path, &mut self.tui_offset)?;
        let mut line = String::new();
        while reader.read_line(&mut line)? > 0 {
            let current = line.trim().to_string();
            self.tui_offset += line.len() as u64;
            if self.thread_id.is_none() {
                self.thread_id = extract_thread_id(&current);
                if let Some(thread_id) = &self.thread_id {
                    writer.set_provider_session_id(Some(thread_id.clone()))?;
                    writer.add_source_path(path.to_string_lossy().to_string())?;
                }
            }
            if let Some(thread_id) = &self.thread_id
                && current.contains(thread_id)
            {
                if let Some(event) = parse_codex_tui_line(&current) {
                    writer.emit(LogSourceKind::ProviderLog, event)?;
                }
            }
            line.clear();
        }
        Ok(())
    }

    fn poll_history(&mut self, writer: &SessionLogWriter) -> Result<()> {
        let path = codex_history_path();
        if !path.exists() {
            return Ok(());
        }
        let mut reader = open_reader_from_offset(&path, &mut self.history_offset)?;
        let mut line = String::new();
        while reader.read_line(&mut line)? > 0 {
            self.history_offset += line.len() as u64;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                line.clear();
                continue;
            }
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed)
                && let (Some(session_id), Some(text)) = (
                    value.get("session_id").and_then(|value| value.as_str()),
                    value.get("text").and_then(|value| value.as_str()),
                )
            {
                self.pending_history
                    .push((session_id.to_string(), text.to_string()));
            }
            line.clear();
        }

        if let Some(thread_id) = &self.thread_id {
            let mut still_pending = Vec::new();
            for (session_id, text) in self.pending_history.drain(..) {
                if &session_id == thread_id {
                    writer.emit(
                        LogSourceKind::ProviderLog,
                        LogEventKind::UserMessage {
                            role: "user".to_string(),
                            content: text,
                            message_id: None,
                        },
                    )?;
                } else {
                    still_pending.push((session_id, text));
                }
            }
            self.pending_history = still_pending;
            writer.add_source_path(path.to_string_lossy().to_string())?;
        }

        Ok(())
    }
}

impl HistoricalLogAdapter for CodexHistoricalLogAdapter {
    fn backfill(&self, _root: Option<&str>) -> Result<Vec<BackfilledSession>> {
        let mut sessions = std::collections::HashMap::<String, BackfilledSession>::new();
        let path = codex_history_path();
        if path.exists() {
            info!("Scanning Codex history: {}", path.display());
            let file = std::fs::File::open(&path)?;
            let reader = std::io::BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                let value: serde_json::Value = match serde_json::from_str(&line) {
                    Ok(value) => value,
                    Err(_) => continue,
                };
                let Some(session_id) = value.get("session_id").and_then(|value| value.as_str())
                else {
                    continue;
                };
                let entry =
                    sessions
                        .entry(session_id.to_string())
                        .or_insert_with(|| BackfilledSession {
                            metadata: SessionLogMetadata {
                                provider: "codex".to_string(),
                                wrapper_session_id: session_id.to_string(),
                                provider_session_id: Some(session_id.to_string()),
                                workspace_path: None,
                                command: "backfill".to_string(),
                                model: None,
                                resumed: false,
                                backfilled: true,
                            },
                            completeness: LogCompleteness::Partial,
                            source_paths: vec![path.to_string_lossy().to_string()],
                            events: Vec::new(),
                        });
                if let Some(text) = value.get("text").and_then(|value| value.as_str()) {
                    entry.events.push((
                        LogSourceKind::Backfill,
                        LogEventKind::UserMessage {
                            role: "user".to_string(),
                            content: text.to_string(),
                            message_id: None,
                        },
                    ));
                }
            }
        }

        let tui_path = codex_tui_log_path();
        if tui_path.exists() {
            info!("Scanning Codex TUI log: {}", tui_path.display());
            let file = std::fs::File::open(&tui_path)?;
            let reader = std::io::BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                let Some(thread_id) = extract_thread_id(&line) else {
                    continue;
                };
                if let Some(session) = sessions.get_mut(&thread_id)
                    && let Some(event) = parse_codex_tui_line(&line)
                {
                    session.events.push((LogSourceKind::Backfill, event));
                    if !session
                        .source_paths
                        .contains(&tui_path.to_string_lossy().to_string())
                    {
                        session
                            .source_paths
                            .push(tui_path.to_string_lossy().to_string());
                    }
                }
            }
        }

        Ok(sessions.into_values().collect())
    }
}

fn parse_codex_tui_line(line: &str) -> Option<LogEventKind> {
    if let Some(rest) = line.split("ToolCall: ").nth(1) {
        let mut parts = rest.splitn(2, ' ');
        let tool_name = parts.next()?.to_string();
        let json_part = parts
            .next()
            .unwrap_or_default()
            .split(" thread_id=")
            .next()
            .unwrap_or_default();
        let input = serde_json::from_str(json_part).ok();
        return Some(LogEventKind::ToolCall {
            tool_kind: Some(tool_kind_from_name(&tool_name)),
            tool_name,
            tool_id: None,
            input,
        });
    }

    if line.contains("BackgroundEvent:") || line.contains("codex_core::client:") {
        return Some(LogEventKind::ProviderStatus {
            message: line.to_string(),
            data: None,
        });
    }

    None
}

fn extract_thread_id(line: &str) -> Option<String> {
    let needle = "thread_id=";
    let start = line.find(needle)? + needle.len();
    let tail = &line[start..];
    let end = tail.find([' ', '}', ':']).unwrap_or(tail.len());
    Some(tail[..end].to_string())
}

fn codex_history_path() -> std::path::PathBuf {
    history_path()
}

fn codex_tui_log_path() -> std::path::PathBuf {
    tui_log_path()
}

fn file_len(path: &std::path::Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|metadata| metadata.len())
}

fn open_reader_from_offset(
    path: &std::path::Path,
    offset: &mut u64,
) -> Result<std::io::BufReader<std::fs::File>> {
    let mut file = std::fs::File::open(path)?;
    use std::io::Seek;
    file.seek(std::io::SeekFrom::Start(*offset))?;
    Ok(std::io::BufReader::new(file))
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
            ModelSize::Small => "gpt-5.4-mini",
            ModelSize::Medium => "gpt-5.3-codex",
            ModelSize::Large => "gpt-5.4",
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

    async fn run_resume_with_prompt(
        &self,
        session_id: &str,
        prompt: &str,
    ) -> Result<Option<AgentOutput>> {
        log::debug!(
            "Codex resume with prompt: session={}, prompt={}",
            session_id,
            prompt
        );
        if !self.system_prompt.is_empty() {
            self.write_agents_file().await?;
        }

        let in_sandbox = self.sandbox.is_some();
        let mut args = vec!["exec".to_string(), "--skip-git-repo-check".to_string()];

        if self.output_format.as_deref() == Some("json") {
            args.push("--json".to_string());
        }

        if self.ephemeral {
            args.push("--ephemeral".to_string());
        }

        if !in_sandbox && let Some(ref root) = self.root {
            args.extend(["--cd".to_string(), root.clone()]);
        }

        args.extend(["--model".to_string(), self.model.clone()]);

        for dir in &self.add_dirs {
            args.extend(["--add-dir".to_string(), dir.clone()]);
        }

        if self.skip_permissions {
            args.push("--full-auto".to_string());
        }

        if let Some(turns) = self.max_turns {
            args.extend(["--max-turns".to_string(), turns.to_string()]);
        }

        if let Some(ref schema) = self.output_schema {
            args.extend(["--output-schema".to_string(), schema.clone()]);
        }

        args.extend(["--resume".to_string(), session_id.to_string()]);
        args.push(prompt.to_string());

        let mut cmd = self.make_command(args);
        let raw = crate::process::run_captured(&mut cmd, "Codex").await?;
        Ok(Some(self.build_output(&raw)))
    }

    async fn run_resume(&self, session_id: Option<&str>, last: bool) -> Result<()> {
        let in_sandbox = self.sandbox.is_some();
        let mut args = vec!["resume".to_string()];

        if let Some(id) = session_id {
            args.push(id.to_string());
        } else if last {
            args.push("--last".to_string());
        }

        if !in_sandbox && let Some(ref root) = self.root {
            args.extend(["--cd".to_string(), root.clone()]);
        }

        args.extend(["--model".to_string(), self.model.clone()]);

        for dir in &self.add_dirs {
            args.extend(["--add-dir".to_string(), dir.clone()]);
        }

        if self.skip_permissions {
            args.push("--full-auto".to_string());
        }

        let mut cmd = self.make_command(args);

        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let status = cmd
            .status()
            .await
            .context("Failed to execute 'codex' CLI. Is it installed and in PATH?")?;
        if !status.success() {
            anyhow::bail!("Codex resume failed with status: {}", status);
        }
        Ok(())
    }

    async fn cleanup(&self) -> Result<()> {
        log::debug!("Cleaning up Codex agent resources");
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
