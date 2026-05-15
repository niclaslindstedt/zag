use super::*;
use chrono::Duration;
use std::sync::Mutex;
use tempfile::TempDir;
use zag_agent::session_log::{LogCompleteness, SessionLogMetadata, SessionLogWriter};

/// Test ResumeStrategy that records what it was called with.
struct CapturingStrategy {
    captured: Arc<Mutex<Vec<(String, String, u32)>>>,
    should_fail: bool,
}

impl ResumeStrategy for CapturingStrategy {
    fn resume<'a>(
        &'a self,
        session_id: &'a str,
        message: &'a str,
        attempt: u32,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        let captured = Arc::clone(&self.captured);
        let session_id = session_id.to_string();
        let message = message.to_string();
        let should_fail = self.should_fail;
        Box::pin(async move {
            captured
                .lock()
                .unwrap()
                .push((session_id, message, attempt));
            if should_fail {
                Err(anyhow::anyhow!("simulated resume failure"))
            } else {
                Ok(())
            }
        })
    }
}

fn make_writer(dir: &TempDir, provider: &str) -> Arc<SessionLogWriter> {
    let writer = SessionLogWriter::create(
        dir.path(),
        SessionLogMetadata {
            provider: provider.to_string(),
            wrapper_session_id: "test-session".to_string(),
            provider_session_id: None,
            workspace_path: None,
            command: "test".to_string(),
            model: None,
            resumed: false,
            backfilled: false,
        },
    )
    .expect("create writer");
    writer.set_completeness(LogCompleteness::Full).unwrap();
    Arc::new(writer)
}

fn find_jsonl(dir: &std::path::Path) -> Option<std::path::PathBuf> {
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_jsonl(&path) {
                return Some(found);
            }
        } else if path.extension().is_some_and(|x| x == "jsonl") {
            return Some(path);
        }
    }
    None
}

fn read_events(dir: &TempDir) -> Vec<serde_json::Value> {
    use std::io::BufRead;
    let path = find_jsonl(dir.path()).expect("a jsonl log exists under dir");
    let file = std::fs::File::open(&path).unwrap();
    std::io::BufReader::new(file)
        .lines()
        .map_while(|l| l.ok())
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(&l).ok())
        .collect()
}

#[tokio::test]
async fn schedule_resume_fires_strategy_after_wait() {
    let dir = TempDir::new().unwrap();
    let writer = make_writer(&dir, "claude");
    let captured = Arc::new(Mutex::new(Vec::new()));
    let strategy: Arc<dyn ResumeStrategy> = Arc::new(CapturingStrategy {
        captured: Arc::clone(&captured),
        should_fail: false,
    });

    let when = Utc::now() + Duration::milliseconds(150);
    let handle = schedule_resume(
        "test-session".to_string(),
        when,
        "Continue".to_string(),
        "incident-1".to_string(),
        1,
        Arc::clone(&writer),
        strategy,
    );

    handle.await.unwrap();

    let calls = captured.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "test-session");
    assert_eq!(calls[0].1, "Continue");
    assert_eq!(calls[0].2, 1);

    let events = read_events(&dir);
    let resumed_count = events
        .iter()
        .filter(|e| e.get("type").and_then(|v| v.as_str()) == Some("usage_limit_resumed"))
        .count();
    assert_eq!(resumed_count, 1, "expected one UsageLimitResumed event");
}

#[tokio::test]
async fn schedule_resume_emits_failed_event_on_error() {
    let dir = TempDir::new().unwrap();
    let writer = make_writer(&dir, "codex");
    let captured = Arc::new(Mutex::new(Vec::new()));
    let strategy: Arc<dyn ResumeStrategy> = Arc::new(CapturingStrategy {
        captured: Arc::clone(&captured),
        should_fail: true,
    });

    let when = Utc::now() + Duration::milliseconds(50);
    schedule_resume(
        "test-session".to_string(),
        when,
        "Continue".to_string(),
        "incident-2".to_string(),
        3,
        Arc::clone(&writer),
        strategy,
    )
    .await
    .unwrap();

    let events = read_events(&dir);
    let failed = events
        .iter()
        .find(|e| e.get("type").and_then(|v| v.as_str()) == Some("usage_limit_resume_failed"))
        .expect("expected a UsageLimitResumeFailed event");
    assert_eq!(
        failed.get("incident_id").and_then(|v| v.as_str()),
        Some("incident-2")
    );
    assert_eq!(failed.get("attempt").and_then(|v| v.as_u64()), Some(3));
    assert!(
        failed
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .contains("simulated resume failure")
    );
}

// Smoke test: strategy_for hands back something concrete for each provider.
// We can't easily assert the concrete type through a `dyn Trait` (trait
// objects erase that), so we instead check the FIFO strategy errors out the
// way we'd expect when no FIFO exists (= we got the FIFO path, not the
// respawn path).
#[tokio::test]
async fn strategy_for_claude_uses_fifo_path() {
    let s = strategy_for("claude", None, None);
    // No FIFO has been created for "nonexistent-session", so resume should fail
    // with a message mentioning the FIFO path / relay — proving we took the
    // FifoResumeStrategy branch rather than the respawn one (which would talk
    // about the session store).
    let err = s
        .resume("nonexistent-session", "Continue", 1)
        .await
        .unwrap_err()
        .to_string();
    let lower = err.to_lowercase();
    assert!(
        lower.contains("fifo") || lower.contains("relay") || lower.contains("interactive"),
        "expected FIFO-path error, got: {err}"
    );
}

#[tokio::test]
async fn strategy_for_codex_uses_respawn_path() {
    let s = strategy_for("codex", None, None);
    // No session in the store with that id; the respawn path should error
    // out mentioning the session store / provider_session_id — proving we
    // took the RespawnResumeStrategy branch.
    let err = s
        .resume("nonexistent-session", "Continue", 1)
        .await
        .unwrap_err()
        .to_string();
    let lower = err.to_lowercase();
    assert!(
        lower.contains("session") && (lower.contains("store") || lower.contains("not found")),
        "expected respawn-path error, got: {err}"
    );
}
