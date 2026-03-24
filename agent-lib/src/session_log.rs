use crate::output::{AgentOutput, ContentBlock, Event};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use log::info;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::watch;
use tokio::task::JoinHandle;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LogCompleteness {
    Full,
    Partial,
    MetadataOnly,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LogSourceKind {
    Wrapper,
    ProviderFile,
    ProviderLog,
    Stdout,
    Stderr,
    Backfill,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LogEventKind {
    SessionStarted {
        command: String,
        model: Option<String>,
        cwd: Option<String>,
        resumed: bool,
        backfilled: bool,
    },
    UserMessage {
        role: String,
        content: String,
        message_id: Option<String>,
    },
    AssistantMessage {
        content: String,
        message_id: Option<String>,
    },
    Reasoning {
        content: String,
        message_id: Option<String>,
    },
    ToolCall {
        tool_name: String,
        tool_id: Option<String>,
        input: Option<Value>,
    },
    ToolResult {
        tool_name: Option<String>,
        tool_id: Option<String>,
        success: Option<bool>,
        output: Option<String>,
        error: Option<String>,
        data: Option<Value>,
    },
    Permission {
        tool_name: String,
        description: String,
        granted: bool,
    },
    ProviderStatus {
        message: String,
        data: Option<Value>,
    },
    Stderr {
        message: String,
    },
    ParseWarning {
        message: String,
        raw: Option<String>,
    },
    SessionEnded {
        success: bool,
        error: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLogEvent {
    pub seq: u64,
    pub ts: String,
    pub provider: String,
    pub wrapper_session_id: String,
    #[serde(default)]
    pub provider_session_id: Option<String>,
    pub source_kind: LogSourceKind,
    pub completeness: LogCompleteness,
    #[serde(flatten)]
    pub kind: LogEventKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionLogIndex {
    pub sessions: Vec<SessionLogIndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLogIndexEntry {
    pub wrapper_session_id: String,
    pub provider: String,
    #[serde(default)]
    pub provider_session_id: Option<String>,
    pub log_path: String,
    pub completeness: LogCompleteness,
    pub started_at: String,
    #[serde(default)]
    pub ended_at: Option<String>,
    #[serde(default)]
    pub workspace_path: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub source_paths: Vec<String>,
    #[serde(default)]
    pub backfilled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackfillState {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub imported_session_keys: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SessionLogMetadata {
    pub provider: String,
    pub wrapper_session_id: String,
    pub provider_session_id: Option<String>,
    pub workspace_path: Option<String>,
    pub command: String,
    pub model: Option<String>,
    pub resumed: bool,
    pub backfilled: bool,
}

#[derive(Debug, Clone)]
pub struct LiveLogContext {
    pub root: Option<String>,
    pub provider_session_id: Option<String>,
    pub workspace_path: Option<String>,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct BackfilledSession {
    pub metadata: SessionLogMetadata,
    pub completeness: LogCompleteness,
    pub source_paths: Vec<String>,
    pub events: Vec<(LogSourceKind, LogEventKind)>,
}

#[async_trait]
pub trait LiveLogAdapter: Send {
    async fn poll(&mut self, writer: &SessionLogWriter) -> Result<()>;

    async fn finalize(&mut self, writer: &SessionLogWriter) -> Result<()> {
        self.poll(writer).await
    }
}

pub trait HistoricalLogAdapter: Send + Sync {
    fn backfill(&self, root: Option<&str>) -> Result<Vec<BackfilledSession>>;
}

#[derive(Clone)]
pub struct SessionLogWriter {
    state: Arc<Mutex<WriterState>>,
}

struct WriterState {
    metadata: SessionLogMetadata,
    log_path: PathBuf,
    index_path: PathBuf,
    next_seq: u64,
    completeness: LogCompleteness,
}

pub struct SessionLogCoordinator {
    writer: SessionLogWriter,
    stop_tx: Option<watch::Sender<bool>>,
    task: Option<JoinHandle<Result<()>>>,
}

impl SessionLogWriter {
    /// Create a new session log writer.
    ///
    /// `logs_dir` is the base directory for session logs (e.g. `~/.agent/projects/<path>/logs`).
    /// The writer will create a `sessions/` subdirectory under it for JSONL log files
    /// and an `index.json` file for session metadata.
    pub fn create(logs_dir: &Path, metadata: SessionLogMetadata) -> Result<Self> {
        let sessions_dir = logs_dir.join("sessions");
        std::fs::create_dir_all(&sessions_dir).with_context(|| {
            format!(
                "Failed to create session log directory: {}",
                sessions_dir.display()
            )
        })?;
        let log_path = sessions_dir.join(format!("{}.jsonl", metadata.wrapper_session_id));
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }
        if !log_path.exists() {
            File::create(&log_path)
                .with_context(|| format!("Failed to create log file: {}", log_path.display()))?;
        }

        let next_seq = next_sequence(&log_path)?;
        let index_path = logs_dir.join("index.json");
        let writer = Self {
            state: Arc::new(Mutex::new(WriterState {
                metadata: metadata.clone(),
                log_path: log_path.clone(),
                index_path,
                next_seq,
                completeness: LogCompleteness::Full,
            })),
        };

        writer.upsert_index()?;
        Ok(writer)
    }

    pub fn log_path(&self) -> Result<PathBuf> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("Log mutex poisoned"))?;
        Ok(state.log_path.clone())
    }

    pub fn set_provider_session_id(&self, provider_session_id: Option<String>) -> Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("Log mutex poisoned"))?;
        state.metadata.provider_session_id = provider_session_id;
        drop(state);
        self.upsert_index()
    }

    pub fn set_completeness(&self, completeness: LogCompleteness) -> Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("Log mutex poisoned"))?;
        if rank_completeness(completeness) < rank_completeness(state.completeness) {
            state.completeness = completeness;
        }
        drop(state);
        self.upsert_index()
    }

    pub fn add_source_path(&self, path: impl Into<String>) -> Result<()> {
        let path = path.into();
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("Log mutex poisoned"))?;
        let wrapper_session_id = state.metadata.wrapper_session_id.clone();
        let index_path = state.index_path.clone();
        drop(state);

        let mut index = load_index(&index_path)?;
        if let Some(entry) = index
            .sessions
            .iter_mut()
            .find(|entry| entry.wrapper_session_id == wrapper_session_id)
            && !entry.source_paths.contains(&path)
        {
            entry.source_paths.push(path);
            save_index(&index_path, &index)?;
        }
        Ok(())
    }

    pub fn emit(&self, source_kind: LogSourceKind, kind: LogEventKind) -> Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("Log mutex poisoned"))?;
        let event = AgentLogEvent {
            seq: state.next_seq,
            ts: Utc::now().to_rfc3339(),
            provider: state.metadata.provider.clone(),
            wrapper_session_id: state.metadata.wrapper_session_id.clone(),
            provider_session_id: state.metadata.provider_session_id.clone(),
            source_kind,
            completeness: state.completeness,
            kind,
        };
        state.next_seq += 1;

        let mut file = OpenOptions::new()
            .append(true)
            .open(&state.log_path)
            .with_context(|| format!("Failed to open {}", state.log_path.display()))?;
        writeln!(file, "{}", serde_json::to_string(&event)?)
            .with_context(|| format!("Failed to write {}", state.log_path.display()))?;
        Ok(())
    }

    pub fn finish(&self, success: bool, error: Option<String>) -> Result<()> {
        self.emit(
            LogSourceKind::Wrapper,
            LogEventKind::SessionEnded { success, error },
        )?;
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("Log mutex poisoned"))?;
        let index_path = state.index_path.clone();
        let wrapper_session_id = state.metadata.wrapper_session_id.clone();
        drop(state);
        let mut index = load_index(&index_path)?;
        if let Some(entry) = index
            .sessions
            .iter_mut()
            .find(|entry| entry.wrapper_session_id == wrapper_session_id)
        {
            entry.ended_at = Some(Utc::now().to_rfc3339());
        }
        save_index(&index_path, &index)
    }

    fn upsert_index(&self) -> Result<()> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("Log mutex poisoned"))?;
        let mut index = load_index(&state.index_path)?;
        let existing = index
            .sessions
            .iter_mut()
            .find(|entry| entry.wrapper_session_id == state.metadata.wrapper_session_id);
        if let Some(entry) = existing {
            entry.provider_session_id = state.metadata.provider_session_id.clone();
            entry.log_path = state.log_path.to_string_lossy().to_string();
            entry.workspace_path = state.metadata.workspace_path.clone();
            entry.command = Some(state.metadata.command.clone());
            entry.completeness = state.completeness;
            entry.backfilled = state.metadata.backfilled;
        } else {
            index.sessions.push(SessionLogIndexEntry {
                wrapper_session_id: state.metadata.wrapper_session_id.clone(),
                provider: state.metadata.provider.clone(),
                provider_session_id: state.metadata.provider_session_id.clone(),
                log_path: state.log_path.to_string_lossy().to_string(),
                completeness: state.completeness,
                started_at: Utc::now().to_rfc3339(),
                ended_at: None,
                workspace_path: state.metadata.workspace_path.clone(),
                command: Some(state.metadata.command.clone()),
                source_paths: Vec::new(),
                backfilled: state.metadata.backfilled,
            });
        }
        save_index(&state.index_path, &index)
    }
}

impl SessionLogCoordinator {
    /// Start a new session log coordinator.
    ///
    /// `logs_dir` is the base directory for session logs (e.g. `~/.agent/projects/<path>/logs`).
    pub fn start(
        logs_dir: &Path,
        metadata: SessionLogMetadata,
        live_adapter: Option<Box<dyn LiveLogAdapter>>,
    ) -> Result<Self> {
        let writer = SessionLogWriter::create(logs_dir, metadata.clone())?;
        writer.emit(
            if metadata.backfilled {
                LogSourceKind::Backfill
            } else {
                LogSourceKind::Wrapper
            },
            LogEventKind::SessionStarted {
                command: metadata.command.clone(),
                model: metadata.model.clone(),
                cwd: metadata.workspace_path.clone(),
                resumed: metadata.resumed,
                backfilled: metadata.backfilled,
            },
        )?;

        if let Some(adapter) = live_adapter {
            let (stop_tx, stop_rx) = watch::channel(false);
            let writer_clone = writer.clone();
            let task =
                tokio::spawn(async move { run_live_adapter(adapter, writer_clone, stop_rx).await });
            Ok(Self {
                writer,
                stop_tx: Some(stop_tx),
                task: Some(task),
            })
        } else {
            Ok(Self {
                writer,
                stop_tx: None,
                task: None,
            })
        }
    }

    pub fn writer(&self) -> &SessionLogWriter {
        &self.writer
    }

    pub async fn finish(mut self, success: bool, error: Option<String>) -> Result<()> {
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(true);
        }
        if let Some(task) = self.task.take() {
            task.await??;
        }
        self.writer.finish(success, error)
    }
}

pub fn record_prompt(writer: &SessionLogWriter, prompt: Option<&str>) -> Result<()> {
    if let Some(prompt) = prompt
        && !prompt.trim().is_empty()
    {
        writer.emit(
            LogSourceKind::Wrapper,
            LogEventKind::UserMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
                message_id: None,
            },
        )?;
    }
    Ok(())
}

pub fn record_agent_output(writer: &SessionLogWriter, output: &AgentOutput) -> Result<()> {
    if !output.session_id.is_empty() && output.session_id != "unknown" {
        writer.set_provider_session_id(Some(output.session_id.clone()))?;
    }
    for event in &output.events {
        match event {
            Event::AssistantMessage { content, .. } => {
                for block in content {
                    match block {
                        ContentBlock::Text { text } => {
                            writer.emit(
                                LogSourceKind::Wrapper,
                                LogEventKind::AssistantMessage {
                                    content: text.clone(),
                                    message_id: None,
                                },
                            )?;
                        }
                        ContentBlock::ToolUse { id, name, input } => {
                            writer.emit(
                                LogSourceKind::Wrapper,
                                LogEventKind::ToolCall {
                                    tool_name: name.clone(),
                                    tool_id: Some(id.clone()),
                                    input: Some(input.clone()),
                                },
                            )?;
                        }
                    }
                }
            }
            Event::ToolExecution {
                tool_name,
                tool_id,
                result,
                ..
            } => {
                writer.emit(
                    LogSourceKind::Wrapper,
                    LogEventKind::ToolResult {
                        tool_name: Some(tool_name.clone()),
                        tool_id: Some(tool_id.clone()),
                        success: Some(result.success),
                        output: result.output.clone(),
                        error: result.error.clone(),
                        data: result.data.clone(),
                    },
                )?;
            }
            Event::PermissionRequest {
                tool_name,
                description,
                granted,
            } => {
                writer.emit(
                    LogSourceKind::Wrapper,
                    LogEventKind::Permission {
                        tool_name: tool_name.clone(),
                        description: description.clone(),
                        granted: *granted,
                    },
                )?;
            }
            Event::Error { message, details } => {
                writer.emit(
                    LogSourceKind::Wrapper,
                    LogEventKind::ProviderStatus {
                        message: message.clone(),
                        data: details.clone(),
                    },
                )?;
            }
            Event::Init {
                model,
                working_directory,
                metadata,
                ..
            } => {
                writer.emit(
                    LogSourceKind::Wrapper,
                    LogEventKind::ProviderStatus {
                        message: format!("Initialized {}", model),
                        data: Some(serde_json::json!({
                            "working_directory": working_directory,
                            "metadata": metadata,
                        })),
                    },
                )?;
            }
            Event::Result {
                success,
                message,
                duration_ms,
                num_turns,
            } => {
                writer.emit(
                    LogSourceKind::Wrapper,
                    LogEventKind::ProviderStatus {
                        message: message
                            .clone()
                            .unwrap_or_else(|| "Result emitted".to_string()),
                        data: Some(serde_json::json!({
                            "success": success,
                            "duration_ms": duration_ms,
                            "num_turns": num_turns,
                        })),
                    },
                )?;
            }
        }
    }
    Ok(())
}

/// Run historical log backfill from the given provider adapters.
///
/// `logs_dir` is the base directory for session logs.
pub fn run_backfill(
    logs_dir: &Path,
    root: Option<&str>,
    providers: &[&dyn HistoricalLogAdapter],
) -> Result<usize> {
    let state_path = logs_dir.join("backfill_state.json");
    let mut state = load_backfill_state(&state_path)?;
    let current_version = 1;
    if state.version == current_version {
        info!(
            "Historical log import already completed for version {}",
            current_version
        );
        return Ok(0);
    }

    info!("Starting historical log import");
    let mut imported = 0;
    for provider in providers {
        for session in provider.backfill(root)? {
            let key = session_key(&session.metadata);
            if state.imported_session_keys.contains(&key) {
                info!(
                    "Skipping already imported historical session: {} {}",
                    session.metadata.provider,
                    session
                        .metadata
                        .provider_session_id
                        .as_deref()
                        .unwrap_or(&session.metadata.wrapper_session_id)
                );
                continue;
            }

            info!(
                "Importing historical session: {} {}",
                session.metadata.provider,
                session
                    .metadata
                    .provider_session_id
                    .as_deref()
                    .unwrap_or(&session.metadata.wrapper_session_id)
            );

            let writer = SessionLogWriter::create(logs_dir, session.metadata.clone())?;
            writer.set_completeness(session.completeness)?;
            for source_path in session.source_paths {
                info!("  source: {}", source_path);
                let _ = writer.add_source_path(source_path);
            }
            for (source_kind, event) in session.events {
                writer.emit(source_kind, event)?;
            }
            writer.finish(true, None)?;
            state.imported_session_keys.push(key);
            imported += 1;
        }
    }

    state.version = current_version;
    save_backfill_state(&state_path, &state)?;
    info!(
        "Historical log import finished: {} session(s) imported",
        imported
    );
    Ok(imported)
}

async fn run_live_adapter(
    mut adapter: Box<dyn LiveLogAdapter>,
    writer: SessionLogWriter,
    mut stop_rx: watch::Receiver<bool>,
) -> Result<()> {
    loop {
        adapter.poll(&writer).await?;
        tokio::select! {
            changed = stop_rx.changed() => {
                if changed.is_ok() && *stop_rx.borrow() {
                    break;
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(250)) => {}
        }
    }
    adapter.finalize(&writer).await
}

fn next_sequence(path: &Path) -> Result<u64> {
    if !path.exists() {
        return Ok(1);
    }
    let file = File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut last_seq = 0;
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(&line)
            && let Some(seq) = value.get("seq").and_then(|seq| seq.as_u64())
        {
            last_seq = seq;
        }
    }
    Ok(last_seq + 1)
}

fn load_index(path: &Path) -> Result<SessionLogIndex> {
    if !path.exists() {
        return Ok(SessionLogIndex::default());
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    Ok(serde_json::from_str(&content).unwrap_or_default())
}

fn save_index(path: &Path, index: &SessionLogIndex) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    std::fs::write(path, serde_json::to_string_pretty(index)?)
        .with_context(|| format!("Failed to write {}", path.display()))
}

fn load_backfill_state(path: &Path) -> Result<BackfillState> {
    if !path.exists() {
        return Ok(BackfillState::default());
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    Ok(serde_json::from_str(&content).unwrap_or_default())
}

fn save_backfill_state(path: &Path, state: &BackfillState) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    std::fs::write(path, serde_json::to_string_pretty(state)?)
        .with_context(|| format!("Failed to write {}", path.display()))
}

fn rank_completeness(completeness: LogCompleteness) -> u8 {
    match completeness {
        LogCompleteness::Full => 3,
        LogCompleteness::Partial => 2,
        LogCompleteness::MetadataOnly => 1,
    }
}

fn session_key(metadata: &SessionLogMetadata) -> String {
    format!(
        "{}:{}",
        metadata.provider,
        metadata
            .provider_session_id
            .as_deref()
            .unwrap_or(&metadata.wrapper_session_id)
    )
}

#[cfg(test)]
#[path = "session_log_tests.rs"]
mod tests;
