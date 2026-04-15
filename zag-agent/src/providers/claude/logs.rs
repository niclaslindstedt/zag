use crate::session_log::{
    BackfilledSession, HistoricalLogAdapter, LiveLogAdapter, LiveLogContext, LogCompleteness,
    LogEventKind, LogSourceKind, SessionLogMetadata, SessionLogWriter, ToolKind,
};

/// Classify a Claude Code built-in tool name into a normalized ToolKind.
fn tool_kind_from_name(name: &str) -> ToolKind {
    match name {
        "Bash" => ToolKind::Shell,
        "Read" => ToolKind::FileRead,
        "Write" => ToolKind::FileWrite,
        "Edit" => ToolKind::FileEdit,
        "Glob" | "Grep" => ToolKind::Search,
        "Agent" => ToolKind::SubAgent,
        "WebFetch" | "WebSearch" => ToolKind::Web,
        "NotebookEdit" => ToolKind::Notebook,
        _ => ToolKind::Other,
    }
}
use anyhow::{Context, Result};
use async_trait::async_trait;
use log::info;
use serde_json::Value;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

pub struct ClaudeLiveLogAdapter {
    ctx: LiveLogContext,
    session_path: Option<PathBuf>,
    offset: u64,
    seen_keys: HashSet<String>,
    /// Track the current provider session ID so we can detect when it changes.
    current_provider_session_id: Option<String>,
}

pub struct ClaudeHistoricalLogAdapter;

impl ClaudeLiveLogAdapter {
    pub fn new(ctx: LiveLogContext) -> Self {
        let current_provider_session_id = ctx.provider_session_id.clone();
        Self {
            ctx,
            session_path: None,
            offset: 0,
            seen_keys: HashSet::new(),
            current_provider_session_id,
        }
    }

    /// Check if a newer session file has appeared for the same workspace.
    /// Only called when `is_worktree` is true, since the unique workspace path
    /// makes detection reliable.
    fn detect_newer_session(&self) -> Option<PathBuf> {
        let current_path = self.session_path.as_ref()?;
        let current_modified = std::fs::metadata(current_path).ok()?.modified().ok()?;
        let workspace = self.ctx.workspace_path.as_deref()?;
        let projects_dir = claude_projects_dir()?;

        let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
        let entries = std::fs::read_dir(projects_dir).ok()?;
        for project in entries.flatten() {
            let files = match std::fs::read_dir(project.path()) {
                Ok(files) => files,
                Err(_) => continue,
            };
            for file in files.flatten() {
                let path = file.path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
                    continue;
                }
                // Skip the file we're already tailing
                if path == *current_path {
                    continue;
                }
                let metadata = match file.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                let modified = match metadata.modified() {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                // Only consider files newer than the current session file
                if modified <= current_modified {
                    continue;
                }
                if !file_contains_workspace(&path, workspace) {
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

    fn discover_session_path(&self) -> Option<PathBuf> {
        let projects_dir = claude_projects_dir()?;
        if let Some(session_id) = &self.ctx.provider_session_id {
            if let Ok(projects) = std::fs::read_dir(&projects_dir) {
                for project in projects.flatten() {
                    let candidate = project.path().join(format!("{session_id}.jsonl"));
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }
            }
        }

        let workspace = self.ctx.workspace_path.as_deref();
        let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
        if let Ok(projects) = std::fs::read_dir(projects_dir) {
            for project in projects.flatten() {
                let files = match std::fs::read_dir(project.path()) {
                    Ok(files) => files,
                    Err(_) => continue,
                };
                for file in files.flatten() {
                    let path = file.path();
                    if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
                        continue;
                    }
                    let metadata = match file.metadata() {
                        Ok(metadata) => metadata,
                        Err(_) => continue,
                    };
                    let modified = match metadata.modified() {
                        Ok(modified) => modified,
                        Err(_) => continue,
                    };
                    let started_at = system_time_from_utc(self.ctx.started_at);
                    if modified < started_at {
                        continue;
                    }
                    if let Some(workspace) = workspace
                        && !file_contains_workspace(&path, workspace)
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
            }
        }

        best.map(|(_, path)| path)
    }
}

#[async_trait]
impl LiveLogAdapter for ClaudeLiveLogAdapter {
    async fn poll(&mut self, writer: &SessionLogWriter) -> Result<()> {
        if self.session_path.is_none() {
            self.session_path = self.discover_session_path();
            if let Some(path) = &self.session_path {
                writer.add_source_path(path.to_string_lossy().to_string())?;
            }
        }

        // In worktree mode, check if a newer session file appeared (session clear/restart)
        if self.ctx.is_worktree
            && self.session_path.is_some()
            && let Some(newer_path) = self.detect_newer_session()
        {
            let old_session_id = self.current_provider_session_id.clone();
            log::info!(
                "Session clear detected: new file {} (old: {})",
                newer_path.display(),
                self.session_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            );

            // Reset state for the new session file
            self.session_path = Some(newer_path.clone());
            self.offset = 0;
            self.seen_keys.clear();
            self.current_provider_session_id = None;

            writer.add_source_path(newer_path.to_string_lossy().to_string())?;
            writer.emit(
                LogSourceKind::ProviderFile,
                LogEventKind::SessionCleared {
                    old_session_id,
                    new_session_id: None, // will be discovered when we read the new file
                },
            )?;
        }

        let Some(path) = self.session_path.as_ref() else {
            return Ok(());
        };

        let mut file =
            File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
        file.seek(SeekFrom::Start(self.offset))?;
        let mut reader = BufReader::new(file);
        let mut buf = String::new();

        loop {
            buf.clear();
            let bytes = reader.read_line(&mut buf)?;
            if bytes == 0 {
                break;
            }
            self.offset += bytes as u64;
            let trimmed = buf.trim();
            if trimmed.is_empty() {
                continue;
            }
            let value: Value = match serde_json::from_str(trimmed) {
                Ok(value) => value,
                Err(_) => {
                    writer.emit(
                        LogSourceKind::ProviderFile,
                        LogEventKind::ParseWarning {
                            message: "Failed to parse Claude session line".to_string(),
                            raw: Some(trimmed.to_string()),
                        },
                    )?;
                    continue;
                }
            };
            for event in parse_claude_value(&value, &mut self.seen_keys) {
                writer.emit(LogSourceKind::ProviderFile, event)?;
            }
            if let Some(session_id) = value
                .get("sessionId")
                .or_else(|| value.get("session_id"))
                .and_then(|value| value.as_str())
            {
                self.current_provider_session_id = Some(session_id.to_string());
                writer.set_provider_session_id(Some(session_id.to_string()))?;
            }
        }

        Ok(())
    }
}

impl HistoricalLogAdapter for ClaudeHistoricalLogAdapter {
    fn backfill(&self, _root: Option<&str>) -> Result<Vec<BackfilledSession>> {
        let mut sessions = Vec::new();
        let Some(projects_dir) = claude_projects_dir() else {
            return Ok(sessions);
        };

        let projects = match std::fs::read_dir(projects_dir) {
            Ok(projects) => projects,
            Err(_) => return Ok(sessions),
        };

        for project in projects.flatten() {
            let files = match std::fs::read_dir(project.path()) {
                Ok(files) => files,
                Err(_) => continue,
            };
            for file in files.flatten() {
                let path = file.path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
                    continue;
                }
                info!("Scanning Claude history: {}", path.display());
                if let Some(session) = backfill_session(&path)? {
                    sessions.push(session);
                }
            }
        }

        Ok(sessions)
    }
}

fn backfill_session(path: &Path) -> Result<Option<BackfilledSession>> {
    let file = File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut seen = HashSet::new();
    let mut events = Vec::new();
    let mut provider_session_id = None;
    let mut workspace_path = None;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if provider_session_id.is_none() {
            provider_session_id = value
                .get("sessionId")
                .or_else(|| value.get("session_id"))
                .and_then(|value| value.as_str())
                .map(str::to_string);
        }
        if workspace_path.is_none() {
            workspace_path = value
                .get("cwd")
                .and_then(|value| value.as_str())
                .map(str::to_string);
        }
        for event in parse_claude_value(&value, &mut seen) {
            events.push((LogSourceKind::Backfill, event));
        }
    }

    let Some(provider_session_id) = provider_session_id else {
        return Ok(None);
    };

    Ok(Some(BackfilledSession {
        metadata: SessionLogMetadata {
            provider: "claude".to_string(),
            wrapper_session_id: provider_session_id.clone(),
            provider_session_id: Some(provider_session_id),
            workspace_path,
            command: "backfill".to_string(),
            model: None,
            resumed: false,
            backfilled: true,
        },
        completeness: LogCompleteness::Full,
        source_paths: vec![path.to_string_lossy().to_string()],
        events,
    }))
}

fn parse_claude_value(value: &Value, seen_keys: &mut HashSet<String>) -> Vec<LogEventKind> {
    let mut events = Vec::new();
    let Some(key) = event_key(value) else {
        return events;
    };
    if !seen_keys.insert(key) {
        return events;
    }

    match value.get("type").and_then(|value| value.as_str()) {
        Some("user") => {
            if let Some(content) = value
                .get("message")
                .and_then(|message| message.get("content"))
                .and_then(|content| content.as_str())
            {
                events.push(LogEventKind::UserMessage {
                    role: "user".to_string(),
                    content: content.to_string(),
                    message_id: value
                        .get("uuid")
                        .and_then(|uuid| uuid.as_str())
                        .map(str::to_string),
                });
            } else if let Some(content) = value
                .get("message")
                .and_then(|message| message.get("content"))
                .and_then(|content| content.as_array())
            {
                for block in content {
                    if block.get("type").and_then(|value| value.as_str()) == Some("tool_result") {
                        events.push(LogEventKind::ToolResult {
                            tool_name: None,
                            tool_kind: None, // tool_use_id could be correlated, but name isn't in result
                            tool_id: block
                                .get("tool_use_id")
                                .and_then(|value| value.as_str())
                                .map(str::to_string),
                            success: block
                                .get("is_error")
                                .and_then(|value| value.as_bool())
                                .map(|is_error| !is_error),
                            output: block
                                .get("content")
                                .and_then(|value| value.as_str())
                                .map(str::to_string),
                            error: None,
                            data: value.get("tool_use_result").cloned(),
                        });
                    }
                }
            }
        }
        Some("assistant") => {
            if let Some(content) = value
                .get("message")
                .and_then(|message| message.get("content"))
                .and_then(|content| content.as_array())
            {
                let message_id = value
                    .get("message")
                    .and_then(|message| message.get("id"))
                    .and_then(|id| id.as_str())
                    .map(str::to_string);
                for block in content {
                    match block.get("type").and_then(|value| value.as_str()) {
                        Some("text") => events.push(LogEventKind::AssistantMessage {
                            content: block
                                .get("text")
                                .and_then(|value| value.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            message_id: message_id.clone(),
                        }),
                        Some("thinking") => events.push(LogEventKind::Reasoning {
                            content: block
                                .get("thinking")
                                .and_then(|value| value.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            message_id: message_id.clone(),
                        }),
                        Some("tool_use") => {
                            let name = block
                                .get("name")
                                .and_then(|value| value.as_str())
                                .unwrap_or("unknown");
                            events.push(LogEventKind::ToolCall {
                                tool_kind: Some(tool_kind_from_name(name)),
                                tool_name: name.to_string(),
                                tool_id: block
                                    .get("id")
                                    .and_then(|value| value.as_str())
                                    .map(str::to_string),
                                input: block.get("input").cloned(),
                            });
                        }
                        _ => {}
                    }
                }
            }
        }
        Some("system") => {
            events.push(LogEventKind::ProviderStatus {
                message: "Claude system event".to_string(),
                data: Some(value.clone()),
            });
        }
        Some("result") => {
            if let Some(denials) = value
                .get("permission_denials")
                .and_then(|value| value.as_array())
            {
                for denial in denials {
                    events.push(LogEventKind::Permission {
                        tool_name: denial
                            .get("tool_name")
                            .and_then(|value| value.as_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        description: serde_json::to_string(
                            denial.get("tool_input").unwrap_or(&Value::Null),
                        )
                        .unwrap_or_default(),
                        granted: false,
                    });
                }
            }
            events.push(LogEventKind::ProviderStatus {
                message: value
                    .get("result")
                    .and_then(|result| result.as_str())
                    .unwrap_or("Claude result")
                    .to_string(),
                data: Some(value.clone()),
            });
        }
        Some("queue-operation") | Some("last-prompt") => {
            events.push(LogEventKind::ProviderStatus {
                message: value
                    .get("type")
                    .and_then(|value| value.as_str())
                    .unwrap_or("claude_event")
                    .to_string(),
                data: Some(value.clone()),
            });
        }
        _ => {}
    }

    events
}

fn event_key(value: &Value) -> Option<String> {
    value
        .get("uuid")
        .and_then(|uuid| uuid.as_str())
        .map(str::to_string)
        .or_else(|| {
            Some(format!(
                "{}:{}:{}",
                value
                    .get("timestamp")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
                value
                    .get("type")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
                value
                    .get("sessionId")
                    .or_else(|| value.get("session_id"))
                    .and_then(|value| value.as_str())
                    .unwrap_or("")
            ))
        })
}

fn claude_projects_dir() -> Option<PathBuf> {
    super::projects_dir()
}

fn file_contains_workspace(path: &Path, workspace: &str) -> bool {
    let Ok(file) = File::open(path) else {
        return false;
    };
    let reader = BufReader::new(file);
    reader
        .lines()
        .take(8)
        .flatten()
        .any(|line| line.contains(workspace))
}

fn system_time_from_utc(ts: chrono::DateTime<chrono::Utc>) -> std::time::SystemTime {
    std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(ts.timestamp().max(0) as u64)
}

#[cfg(test)]
#[path = "logs_tests.rs"]
mod tests;
