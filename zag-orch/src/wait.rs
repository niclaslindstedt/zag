//! Wait command: block until one or more sessions complete.

use crate::listen;
use crate::ps::resolve_live_status;
use anyhow::{Result, bail};
use log::debug;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use zag::process_store::ProcessStore;
use zag::session::SessionStore;
use zag::session_log::{AgentLogEvent, LogEventKind};

/// Result of waiting for a single session.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WaitResult {
    pub session_id: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Parameters for the wait command.
pub struct WaitParams {
    /// Session IDs to wait for.
    pub session_ids: Vec<String>,
    /// Filter by tag (wait for all sessions with this tag).
    pub tag: Option<String>,
    /// Wait for the latest session.
    pub latest: bool,
    /// Timeout duration (e.g., "30s", "5m", "1h").
    pub timeout: Option<String>,
    /// Exit on first completed session instead of waiting for all.
    pub any: bool,
    /// Output as JSON.
    pub json: bool,
    /// Root directory for session resolution.
    pub root: Option<String>,
}

/// Parse a duration string like "30s", "5m", "1h", "10m30s".
fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();
    let mut total_secs: u64 = 0;
    let mut current_num = String::new();

    for ch in s.chars() {
        if ch.is_ascii_digit() {
            current_num.push(ch);
        } else {
            let n: u64 = current_num
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid duration: '{}'", s))?;
            current_num.clear();
            match ch {
                's' => total_secs += n,
                'm' => total_secs += n * 60,
                'h' => total_secs += n * 3600,
                'd' => total_secs += n * 86400,
                _ => bail!("Invalid duration unit '{}' in '{}'", ch, s),
            }
        }
    }

    // If there's a trailing number with no unit, treat as seconds
    if !current_num.is_empty() {
        let n: u64 = current_num
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid duration: '{}'", s))?;
        total_secs += n;
    }

    if total_secs == 0 {
        bail!("Duration must be greater than zero: '{}'", s);
    }

    Ok(Duration::from_secs(total_secs))
}

/// Resolve the list of session IDs to wait for.
fn resolve_session_ids(params: &WaitParams) -> Result<Vec<String>> {
    let mut ids = Vec::new();

    if params.latest {
        let store = SessionStore::load(params.root.as_deref())?;
        let entry = store
            .latest()
            .ok_or_else(|| anyhow::anyhow!("No sessions found"))?;
        ids.push(entry.session_id.clone());
    }

    if let Some(ref tag) = params.tag {
        let store = SessionStore::load(params.root.as_deref())?;
        let tagged = store.find_by_tag(tag);
        if tagged.is_empty() {
            bail!("No sessions found with tag '{}'", tag);
        }
        for entry in tagged {
            if !ids.contains(&entry.session_id) {
                ids.push(entry.session_id.clone());
            }
        }
    }

    for id in &params.session_ids {
        if !ids.contains(id) {
            ids.push(id.clone());
        }
    }

    if ids.is_empty() {
        bail!("No sessions specified. Provide session IDs, --tag, or --latest.");
    }

    Ok(ids)
}

/// Check if a session has already ended by scanning its log file.
/// Returns Some(WaitResult) if ended, None if still running or log not found.
fn check_log_for_ended(session_id: &str, log_path: &PathBuf) -> Option<WaitResult> {
    let file = std::fs::File::open(log_path).ok()?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(event) = serde_json::from_str::<AgentLogEvent>(trimmed) {
            if let LogEventKind::SessionEnded { success, error } = &event.kind {
                return Some(WaitResult {
                    session_id: session_id.to_string(),
                    success: *success,
                    error: error.clone(),
                });
            }
        }
    }
    None
}

/// Check if a session's process is dead (not running, no SessionEnded in log).
fn check_process_dead(session_id: &str) -> bool {
    let store = match ProcessStore::load() {
        Ok(s) => s,
        Err(_) => return false,
    };

    // Find process entry matching this session
    let entry = store
        .processes
        .iter()
        .filter(|e| e.session_id.as_deref() == Some(session_id))
        .max_by(|a, b| a.started_at.cmp(&b.started_at));

    match entry {
        Some(e) => {
            let status = resolve_live_status(e);
            status != "running"
        }
        None => false, // No process entry — can't determine
    }
}

/// Run the wait command.
pub fn run_wait(params: WaitParams) -> Result<()> {
    let session_ids = resolve_session_ids(&params)?;
    let timeout = params.timeout.as_deref().map(parse_duration).transpose()?;

    debug!(
        "Waiting for {} session(s): {:?}, timeout: {:?}, any: {}",
        session_ids.len(),
        session_ids,
        timeout,
        params.any
    );

    let start = Instant::now();
    let mut results: Vec<WaitResult> = Vec::new();
    let mut pending: Vec<String> = session_ids.clone();

    loop {
        // Check timeout
        if let Some(timeout_dur) = timeout {
            if start.elapsed() >= timeout_dur {
                if params.json {
                    // Output results collected so far
                    for r in &results {
                        println!("{}", serde_json::to_string(r)?);
                    }
                    // Mark remaining as timed out
                    for id in &pending {
                        let r = WaitResult {
                            session_id: id.clone(),
                            success: false,
                            error: Some("timeout".to_string()),
                        };
                        println!("{}", serde_json::to_string(&r)?);
                    }
                } else {
                    for r in &results {
                        print_result(r);
                    }
                    for id in &pending {
                        eprintln!("Timed out waiting for session {}", id);
                    }
                }
                std::process::exit(124);
            }
        }

        // Check each pending session
        let mut newly_done = Vec::new();
        for id in &pending {
            // Try to find the log file
            let log_path =
                listen::resolve_session_log(Some(id), false, false, params.root.as_deref());

            if let Ok(ref path) = log_path {
                if let Some(result) = check_log_for_ended(id, path) {
                    newly_done.push(result);
                    continue;
                }
            }

            // No SessionEnded in log — check if process is dead
            if check_process_dead(id) {
                // Process died without a clean SessionEnded
                newly_done.push(WaitResult {
                    session_id: id.clone(),
                    success: false,
                    error: Some("process died without clean exit".to_string()),
                });
            }
        }

        // Process newly completed sessions
        for result in newly_done {
            pending.retain(|id| *id != result.session_id);

            if params.json {
                println!("{}", serde_json::to_string(&result)?);
            }

            let is_any_done = params.any;
            results.push(result);

            if is_any_done {
                if !params.json {
                    for r in &results {
                        print_result(r);
                    }
                }
                let exit_code = if results.iter().all(|r| r.success) {
                    0
                } else {
                    1
                };
                std::process::exit(exit_code);
            }
        }

        // All done?
        if pending.is_empty() {
            if !params.json {
                for r in &results {
                    print_result(r);
                }
            }
            let all_success = results.iter().all(|r| r.success);
            std::process::exit(if all_success { 0 } else { 1 });
        }

        // Poll interval
        std::thread::sleep(Duration::from_millis(500));
    }
}

fn print_result(result: &WaitResult) {
    let status = if result.success {
        "\x1b[32m✓\x1b[0m completed"
    } else {
        "\x1b[31m✗\x1b[0m failed"
    };
    let error_info = result
        .error
        .as_deref()
        .map(|e| format!(": {}", e))
        .unwrap_or_default();
    println!("{} {}{}", result.session_id, status, error_info);
}

#[cfg(test)]
#[path = "wait_tests.rs"]
mod tests;
