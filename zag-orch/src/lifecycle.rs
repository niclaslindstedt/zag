//! Filesystem lifecycle markers for external orchestrators.
//!
//! Writes marker files to `~/.zag/events/` so that non-Rust orchestrators
//! can use inotify/kqueue/filesystem polling to detect session lifecycle events.

use log::debug;
use std::path::PathBuf;
use zag_agent::config::Config;

/// Directory for lifecycle event marker files.
fn events_dir() -> PathBuf {
    Config::global_base_dir().join("events")
}

/// Write a `.started` marker file for a session.
pub fn write_started_marker(session_id: &str) {
    let dir = events_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        debug!("Failed to create events directory: {}", e);
        return;
    }
    let path = dir.join(format!("{}.started", session_id));
    let content = serde_json::json!({
        "session_id": session_id,
        "started_at": chrono::Utc::now().to_rfc3339(),
    });
    if let Err(e) = std::fs::write(&path, content.to_string()) {
        debug!("Failed to write started marker: {}", e);
    }
}

/// Write an `.ended` marker file for a session.
pub fn write_ended_marker(session_id: &str, success: bool, exit_code: Option<i32>) {
    let dir = events_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        debug!("Failed to create events directory: {}", e);
        return;
    }
    let path = dir.join(format!("{}.ended", session_id));
    let content = serde_json::json!({
        "session_id": session_id,
        "success": success,
        "exit_code": exit_code,
        "ended_at": chrono::Utc::now().to_rfc3339(),
    });
    if let Err(e) = std::fs::write(&path, content.to_string()) {
        debug!("Failed to write ended marker: {}", e);
    }
}

/// Prune marker files older than 7 days.
pub fn prune_old_markers() {
    let dir = events_dir();
    if !dir.exists() {
        return;
    }
    let cutoff = std::time::SystemTime::now() - std::time::Duration::from_secs(7 * 86400);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        if let Ok(metadata) = entry.metadata()
            && let Ok(modified) = metadata.modified()
            && modified < cutoff
        {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
