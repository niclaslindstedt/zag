//! GC command: clean up old session data, logs, and process entries.

use anyhow::{Result, bail};
use chrono::Utc;
use zag::config::Config;
use zag::process_store::ProcessStore;

/// Parameters for the gc command.
pub struct GcParams {
    pub force: bool,
    pub older_than: String,
    pub keep_logs: bool,
    pub json: bool,
    pub root: Option<String>,
}

/// Summary of what gc would/did clean.
#[derive(Debug, Default, serde::Serialize)]
struct GcReport {
    process_entries_removed: usize,
    lifecycle_markers_removed: usize,
    spawn_logs_removed: usize,
    session_logs_removed: usize,
    dry_run: bool,
}

/// Parse a duration string like "7d", "30d", "24h" into seconds.
fn parse_duration_secs(s: &str) -> Result<i64> {
    let s = s.trim();
    if let Some(days) = s.strip_suffix('d') {
        let n: i64 = days
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid duration: {}", s))?;
        Ok(n * 86400)
    } else if let Some(hours) = s.strip_suffix('h') {
        let n: i64 = hours
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid duration: {}", s))?;
        Ok(n * 3600)
    } else {
        bail!("Invalid duration '{}'. Use e.g. 7d or 24h.", s);
    }
}

/// Check if a file's modification time is older than the cutoff.
fn is_file_old(path: &std::path::Path, cutoff: std::time::SystemTime) -> bool {
    path.metadata()
        .and_then(|m| m.modified())
        .map(|t| t < cutoff)
        .unwrap_or(false)
}

/// Collect session IDs that are still running/idle (should not be cleaned).
fn live_session_ids(root: Option<&str>) -> std::collections::HashSet<String> {
    let mut live = std::collections::HashSet::new();
    if let Ok(proc_store) = ProcessStore::load() {
        for entry in &proc_store.processes {
            if entry.status == "running" {
                if let Some(ref sid) = entry.session_id {
                    live.insert(sid.clone());
                }
            }
        }
    }
    // Also check session store for any session without a SessionEnded event
    // We'll be conservative: only clean process entries and markers for ended sessions
    let _ = root; // root used for session store loading if needed
    live
}

/// Run the gc command.
pub fn run_gc(params: GcParams) -> Result<()> {
    let threshold_secs = parse_duration_secs(&params.older_than)?;
    let cutoff =
        std::time::SystemTime::now() - std::time::Duration::from_secs(threshold_secs as u64);
    let cutoff_chrono = Utc::now() - chrono::Duration::seconds(threshold_secs);

    let dry_run = !params.force;
    let mut report = GcReport {
        dry_run,
        ..Default::default()
    };

    let live = live_session_ids(params.root.as_deref());

    // 1. Clean dead/exited process entries from processes.json
    if let Ok(mut proc_store) = ProcessStore::load() {
        let before = proc_store.processes.len();
        let to_remove: Vec<String> = proc_store
            .processes
            .iter()
            .filter(|e| {
                e.status != "running"
                    && !live.contains(e.session_id.as_deref().unwrap_or(""))
                    && chrono::DateTime::parse_from_rfc3339(&e.started_at)
                        .map(|dt| dt < cutoff_chrono)
                        .unwrap_or(false)
            })
            .map(|e| e.id.clone())
            .collect();
        report.process_entries_removed = to_remove.len();
        if !dry_run && !to_remove.is_empty() {
            proc_store.processes.retain(|e| !to_remove.contains(&e.id));
            let _ = proc_store.save();
        }
        let _ = before; // suppress unused warning
    }

    // 2. Remove old lifecycle markers from ~/.zag/events/
    let events_dir = Config::global_base_dir().join("events");
    if events_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&events_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && is_file_old(&path, cutoff) {
                    // Check it's not for a live session
                    let fname = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    // Marker files are named <session_id>.started or <session_id>.ended
                    let session_id = fname.to_string();
                    if !live.contains(&session_id) {
                        report.lifecycle_markers_removed += 1;
                        if !dry_run {
                            let _ = std::fs::remove_file(&path);
                        }
                    }
                }
            }
        }
    }

    // 3. Remove old spawn logs from ~/.zag/logs/spawn/
    let spawn_dir = Config::global_base_dir().join("logs").join("spawn");
    if spawn_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&spawn_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && is_file_old(&path, cutoff) {
                    report.spawn_logs_removed += 1;
                    if !dry_run {
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }
    }

    // 4. Remove ended session JSONL files (unless --keep-logs)
    if !params.keep_logs {
        let projects_dir = Config::global_base_dir().join("projects");
        if projects_dir.exists() {
            if let Ok(projects) = std::fs::read_dir(&projects_dir) {
                for project in projects.flatten() {
                    let sessions_dir = project.path().join("logs").join("sessions");
                    if sessions_dir.exists() {
                        report.session_logs_removed +=
                            clean_session_logs(&sessions_dir, cutoff, &live, dry_run);
                    }
                }
            }
        }
    }

    // Output
    if params.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        let action = if dry_run { "Would remove" } else { "Removed" };
        if report.process_entries_removed > 0 {
            println!(
                "{} {} process entries",
                action, report.process_entries_removed
            );
        }
        if report.lifecycle_markers_removed > 0 {
            println!(
                "{} {} lifecycle markers",
                action, report.lifecycle_markers_removed
            );
        }
        if report.spawn_logs_removed > 0 {
            println!("{} {} spawn logs", action, report.spawn_logs_removed);
        }
        if report.session_logs_removed > 0 {
            println!("{} {} session logs", action, report.session_logs_removed);
        }
        let total = report.process_entries_removed
            + report.lifecycle_markers_removed
            + report.spawn_logs_removed
            + report.session_logs_removed;
        if total == 0 {
            println!("Nothing to clean up.");
        } else if dry_run {
            println!("\nRun with --force to actually delete.");
        }
    }

    Ok(())
}

/// Clean old session log JSONL files. Returns count of files removed/would-remove.
fn clean_session_logs(
    sessions_dir: &std::path::Path,
    cutoff: std::time::SystemTime,
    live: &std::collections::HashSet<String>,
    dry_run: bool,
) -> usize {
    let mut count = 0;
    let entries = match std::fs::read_dir(sessions_dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        if !is_file_old(&path, cutoff) {
            continue;
        }
        // Extract session ID from filename
        let session_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if live.contains(&session_id) {
            continue;
        }
        // Check if the log has a SessionEnded event (only clean ended sessions)
        if !has_session_ended(&path) {
            continue;
        }
        count += 1;
        if !dry_run {
            let _ = std::fs::remove_file(&path);
        }
    }
    count
}

/// Check if a JSONL log file contains a SessionEnded event.
fn has_session_ended(path: &std::path::Path) -> bool {
    use std::io::{BufRead, BufReader};
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        // Quick string check before parsing JSON
        if line.contains("\"SessionEnded\"") || line.contains("\"session_ended\"") {
            return true;
        }
    }
    false
}

#[cfg(test)]
#[path = "gc_tests.rs"]
mod tests;
