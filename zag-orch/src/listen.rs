//! Listen command: tail a session's JSONL log file and output parsed events in real-time.
//!
//! The pure formatting/filtering helpers live in `zag_agent::listen` so the
//! library-side streaming builder options can reach them without pulling in
//! `zag-orch` (which depends on `zag-agent`). This module re-exports them
//! and adds the file-tailing / session-resolution logic that belongs at the
//! orchestration layer.

use anyhow::{Context, Result, bail};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use zag_agent::config::Config;
use zag_agent::listen::{format_event_rich, format_event_text, format_ts, matches_filter};
use zag_agent::process_store::ProcessStore;
use zag_agent::session_log::load_global_index;
use zag_agent::session_log::{AgentLogEvent, LogEventKind, SessionLogIndex};

pub use zag_agent::listen::{
    ListenFormat, event_type_name, format_event, parse_listen_format, with_timestamp,
};

/// Resolve a session ID from a process reference (OS PID or zag process UUID).
///
/// Accepts either a numeric OS PID or a zag process UUID (the `id` field).
/// If multiple entries match the same PID (OS PIDs are recycled), the most
/// recently started process is used.
pub fn resolve_session_from_ps(value: &str) -> Result<String> {
    let store = ProcessStore::load().context("Failed to load process store")?;

    // Try to match by zag UUID first (exact string match on `id`)
    let by_id: Vec<_> = store
        .processes
        .iter()
        .filter(|e| e.id == value || e.id.starts_with(value))
        .collect();

    let entry = if !by_id.is_empty() {
        // Pick the most recent among UUID matches
        by_id
            .into_iter()
            .max_by(|a, b| a.started_at.cmp(&b.started_at))
    } else {
        // Try numeric OS PID
        let pid: u32 = value
            .parse()
            .with_context(|| format!("'{value}' is not a valid PID or process UUID"))?;
        let by_pid: Vec<_> = store.processes.iter().filter(|e| e.pid == pid).collect();
        by_pid
            .into_iter()
            .max_by(|a, b| a.started_at.cmp(&b.started_at))
    };

    let entry = entry.ok_or_else(|| anyhow::anyhow!("No process found for '{value}'"))?;

    entry.session_id.clone().ok_or_else(|| {
        anyhow::anyhow!(
            "Process '{}' (pid {}) has no associated session",
            entry.id,
            entry.pid
        )
    })
}

/// Resolve the log file path for a session.
pub fn resolve_session_log(
    session_id: Option<&str>,
    latest: bool,
    active: bool,
    root: Option<&str>,
) -> Result<PathBuf> {
    let logs_dir = crate::util::logs_dir(root);
    let sessions_dir = logs_dir.join("sessions");

    if let Some(id) = session_id {
        // Try direct file path first
        let direct = sessions_dir.join(format!("{id}.jsonl"));
        if direct.exists() {
            return Ok(direct);
        }

        // Try index lookup
        let index_path = logs_dir.join("index.json");
        if index_path.exists() {
            let content = std::fs::read_to_string(&index_path)
                .with_context(|| format!("Failed to read {}", index_path.display()))?;
            let index: SessionLogIndex = serde_json::from_str(&content).unwrap_or_default();
            if let Some(entry) = index.sessions.iter().find(|e| e.wrapper_session_id == id) {
                let path = PathBuf::from(&entry.log_path);
                if path.exists() {
                    return Ok(path);
                }
            }
            // Also try matching by prefix
            let matches: Vec<_> = index
                .sessions
                .iter()
                .filter(|e| e.wrapper_session_id.starts_with(id))
                .collect();
            if matches.len() == 1 {
                let path = PathBuf::from(&matches[0].log_path);
                if path.exists() {
                    return Ok(path);
                }
            } else if matches.len() > 1 {
                bail!(
                    "Ambiguous session ID prefix '{}'. Matches: {}",
                    id,
                    matches
                        .iter()
                        .map(|e| e.wrapper_session_id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }

        // Fallback: check global session index
        if let Some(path) = lookup_global_index_by_id(id) {
            return Ok(path);
        }

        bail!("No session log found for '{id}'");
    }

    if latest {
        return resolve_latest_session(&logs_dir);
    }

    if active {
        return resolve_active_session(&sessions_dir);
    }

    bail!("Specify a session ID, --latest, or --active");
}

/// Find the latest session by `started_at` in the index.
fn resolve_latest_session(logs_dir: &Path) -> Result<PathBuf> {
    let index_path = logs_dir.join("index.json");

    // Try project-scoped index first
    if index_path.exists() {
        let content = std::fs::read_to_string(&index_path)
            .with_context(|| format!("Failed to read {}", index_path.display()))?;
        let index: SessionLogIndex = serde_json::from_str(&content).unwrap_or_default();

        if let Some(newest) = index
            .sessions
            .iter()
            .max_by(|a, b| a.started_at.cmp(&b.started_at))
        {
            let path = PathBuf::from(&newest.log_path);
            if path.exists() {
                return Ok(path);
            }
        }
    }

    // Fallback: check global session index for the latest across all projects
    let global_dir = Config::global_base_dir();
    if let Ok(global_index) = load_global_index(&global_dir)
        && let Some(newest) = global_index
            .sessions
            .iter()
            .max_by(|a, b| a.started_at.cmp(&b.started_at))
    {
        let path = PathBuf::from(&newest.log_path);
        if path.exists() {
            return Ok(path);
        }
    }

    bail!("No session index found. Run an agent session first.");
}

/// Look up a session ID (exact or prefix) in the global session index.
fn lookup_global_index_by_id(id: &str) -> Option<PathBuf> {
    let global_dir = Config::global_base_dir();
    let global_index = load_global_index(&global_dir).ok()?;

    // Exact match
    if let Some(entry) = global_index.sessions.iter().find(|e| e.session_id == id) {
        let path = PathBuf::from(&entry.log_path);
        if path.exists() {
            return Some(path);
        }
    }

    // Prefix match
    let matches: Vec<_> = global_index
        .sessions
        .iter()
        .filter(|e| e.session_id.starts_with(id))
        .collect();
    if matches.len() == 1 {
        let path = PathBuf::from(&matches[0].log_path);
        if path.exists() {
            return Some(path);
        }
    }

    None
}

/// Find the most recently modified `.jsonl` file in the sessions directory.
fn resolve_active_session(sessions_dir: &Path) -> Result<PathBuf> {
    if !sessions_dir.exists() {
        bail!("No sessions directory found. Run an agent session first.");
    }

    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    let entries = std::fs::read_dir(sessions_dir).context("Failed to read sessions directory")?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "jsonl")
            && let Ok(metadata) = entry.metadata()
            && let Ok(modified) = metadata.modified()
            && newest
                .as_ref()
                .map(|(current, _)| modified > *current)
                .unwrap_or(true)
        {
            newest = Some((modified, path));
        }
    }

    newest
        .map(|(_, path)| path)
        .ok_or_else(|| anyhow::anyhow!("No session log files found"))
}

/// Stream session events into a channel (for programmatic consumers like the HTTP server).
/// Spawns a background task that polls the JSONL file and sends events into the channel.
/// The task stops when the receiver is dropped or a SessionEnded event is seen.
pub fn stream_session_events(
    path: &Path,
    filters: Option<Vec<String>>,
) -> Result<tokio::sync::mpsc::Receiver<AgentLogEvent>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<AgentLogEvent>(256);
    let path = path.to_path_buf();

    tokio::spawn(async move {
        let file = match File::open(&path) {
            Ok(f) => f,
            Err(_) => return,
        };
        let mut reader = BufReader::new(file);
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = match reader.read_line(&mut line) {
                Ok(n) => n,
                Err(_) => break,
            };

            if bytes_read > 0 {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Ok(event) = serde_json::from_str::<AgentLogEvent>(trimmed) {
                    let is_ended = matches!(event.kind, LogEventKind::SessionEnded { .. });

                    if matches_filter(&event.kind, filters.as_deref())
                        && tx.send(event).await.is_err()
                    {
                        break; // receiver dropped
                    }

                    if is_ended {
                        break;
                    }
                }
            } else {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                let pos = match reader.stream_position() {
                    Ok(p) => p,
                    Err(_) => break,
                };
                if reader.seek(SeekFrom::Start(pos)).is_err() {
                    break;
                }
            }
        }
    });

    Ok(rx)
}

/// Tail a session log file, printing events as they arrive.
/// Returns when a SessionEnded event is seen or the process is interrupted.
pub fn tail_session_log(
    path: &Path,
    format: ListenFormat,
    show_thinking: bool,
    show_timestamps: bool,
    config: &Config,
    filters: Option<&[String]>,
) -> Result<()> {
    let ts_fmt = config.listen_timestamp_format().to_string();
    let file = File::open(path)
        .with_context(|| format!("Failed to open session log: {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line)?;

        if bytes_read > 0 {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            match format {
                ListenFormat::Json => {
                    // For JSON, parse to check filter, then pass through
                    if let Ok(event) = serde_json::from_str::<AgentLogEvent>(trimmed) {
                        if matches_filter(&event.kind, filters) {
                            println!("{trimmed}");
                        }
                    } else {
                        println!("{trimmed}");
                    }
                }
                ListenFormat::Text | ListenFormat::RichText => {
                    match serde_json::from_str::<AgentLogEvent>(trimmed) {
                        Ok(event) => {
                            if !matches_filter(&event.kind, filters) {
                                // Check if session ended even when filtered out
                                if matches!(event.kind, LogEventKind::SessionEnded { .. }) {
                                    return Ok(());
                                }
                                continue;
                            }
                            let formatted = if format == ListenFormat::RichText {
                                format_event_rich(&event, show_thinking)
                            } else {
                                format_event_text(&event, show_thinking)
                            };
                            if let Some(text) = formatted {
                                if show_timestamps {
                                    let ts_str = if format == ListenFormat::RichText {
                                        format!("\x1b[2m[{}]\x1b[0m", format_ts(&event.ts, &ts_fmt))
                                    } else {
                                        format!("[{}]", format_ts(&event.ts, &ts_fmt))
                                    };
                                    println!("{}", with_timestamp(&ts_str, &text));
                                } else {
                                    println!("{text}");
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("[parse error] {}: {}", e, truncate_for_parse_error(trimmed));
                        }
                    }
                }
            }

            // Check if session ended
            if let Ok(event) = serde_json::from_str::<AgentLogEvent>(trimmed)
                && matches!(event.kind, LogEventKind::SessionEnded { .. })
            {
                return Ok(());
            }
        } else {
            // No new data — poll
            std::thread::sleep(std::time::Duration::from_millis(100));
            // Re-seek to current position to pick up new data appended by another process
            let pos = reader.stream_position()?;
            reader.seek(SeekFrom::Start(pos))?;
        }
    }
}

fn truncate_for_parse_error(s: &str) -> String {
    let s = s.replace('\n', "\\n");
    if s.len() <= 80 {
        s
    } else {
        format!("{}...", &s[..80])
    }
}

#[cfg(test)]
#[path = "listen_tests.rs"]
mod tests;
