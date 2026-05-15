use super::*;
use crate::usage_resume_store::PendingResume;
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

fn make_pending(
    session_id: &str,
    when: DateTime<Utc>,
    incident_id: &str,
    attempt: u32,
) -> PendingResume {
    PendingResume {
        incident_id: incident_id.to_string(),
        session_id: session_id.to_string(),
        provider: "claude".to_string(),
        model: None,
        // `root: None` writes the resume record to the *real* global
        // `~/.zag/scheduled_resumes.jsonl`; tests don't assert on that
        // file, and the foreground/relay code only logs persistence
        // failures. Switching to a temp root would require threading
        // a per-test base dir through Config::agent_dir, out of scope here.
        root: None,
        when,
        message: "Continue".to_string(),
        attempt,
        log_path: std::path::PathBuf::from("/tmp/test.jsonl"),
    }
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
        make_pending("test-session", when, "incident-1", 1),
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
        make_pending("test-session", when, "incident-2", 3),
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

// ---------------------------------------------------------------------------
// Tests for run_with_auto_resume helpers (find_usage_limit_in_output,
// extract_provider_session_id, text-blob fallback).
//
// The full end-to-end loop test against an Agent stub lives in zag-cli where
// `providers::mock` is in scope; here we cover the decision-making logic that
// drives the loop.
// ---------------------------------------------------------------------------

use std::collections::HashMap;
use zag_agent::output::{AgentOutput, ContentBlock, Event};
use zag_agent::usage_limits::UsageLimitConfig;

fn empty_output(provider: &str) -> AgentOutput {
    AgentOutput {
        agent: provider.to_string(),
        session_id: String::new(),
        events: Vec::new(),
        result: None,
        is_error: false,
        exit_code: None,
        error_message: None,
        total_cost_usd: None,
        usage: None,
        model: None,
        provider: Some(provider.to_string()),
        log_path: None,
    }
}

#[test]
fn find_usage_limit_recognizes_explicit_detected_event() {
    let cfg = UsageLimitConfig::default();
    let mut out = empty_output("claude");
    out.events.push(Event::UsageLimitDetected {
        provider: "claude".to_string(),
        scope: "weekly".to_string(),
        reset_at: Some("2030-01-01T00:00:00Z".to_string()),
        raw: Some("Claude AI weekly usage limit reached|1893456000".to_string()),
    });

    let hit = find_usage_limit_in_output(&out, "claude", &cfg).expect("should detect");
    assert_eq!(hit.provider, "claude");
    assert_eq!(hit.scope.as_str(), "weekly");
    assert!(hit.reset_at.is_some());
}

#[test]
fn find_usage_limit_falls_back_to_text_scan_for_codex() {
    let cfg = UsageLimitConfig::default();
    let mut out = empty_output("codex");
    // Simulate Codex from_text output — a single Result event with the limit
    // message embedded.
    out.result = Some(
        "You've hit your usage limit. Please try again at Mar 20th, 2030 3:36 PM.".to_string(),
    );
    out.events.push(Event::Result {
        success: false,
        message: out.result.clone(),
        duration_ms: None,
        num_turns: None,
    });

    let hit = find_usage_limit_in_output(&out, "codex", &cfg).expect("should detect via text scan");
    assert_eq!(hit.provider, "codex");
    assert!(hit.reset_at.is_some());
}

#[test]
fn find_usage_limit_returns_none_when_no_signal() {
    let cfg = UsageLimitConfig::default();
    let mut out = empty_output("codex");
    out.result = Some("All good, no limits hit".to_string());
    out.events.push(Event::AssistantMessage {
        content: vec![ContentBlock::Text {
            text: "hello world".to_string(),
        }],
        usage: None,
        parent_tool_use_id: None,
    });

    assert!(find_usage_limit_in_output(&out, "codex", &cfg).is_none());
}

#[test]
fn find_usage_limit_skips_unknown_providers() {
    let cfg = UsageLimitConfig::default();
    let mut out = empty_output("ollama");
    out.result = Some(
        "Claude AI usage limit reached|1893456000 — but this is an ollama session".to_string(),
    );
    out.events.push(Event::Result {
        success: false,
        message: out.result.clone(),
        duration_ms: None,
        num_turns: None,
    });
    // Ollama isn't wired for auto-resume — no detector → no hit.
    assert!(find_usage_limit_in_output(&out, "ollama", &cfg).is_none());
}

#[test]
fn extract_provider_session_id_prefers_output_field() {
    let mut out = empty_output("codex");
    out.session_id = "thread-abc".to_string();
    out.events.push(Event::Init {
        model: "x".to_string(),
        tools: vec![],
        working_directory: None,
        metadata: HashMap::new(),
    });
    assert_eq!(
        extract_provider_session_id(&out),
        Some("thread-abc".to_string())
    );
}

#[test]
fn extract_provider_session_id_falls_back_to_init_metadata() {
    let mut out = empty_output("claude");
    out.session_id = "unknown".to_string(); // Claude's sentinel
    let mut meta = HashMap::new();
    meta.insert(
        "session_id".to_string(),
        serde_json::Value::String("claude-sid-1".to_string()),
    );
    out.events.push(Event::Init {
        model: "claude-sonnet".to_string(),
        tools: vec![],
        working_directory: None,
        metadata: meta,
    });
    assert_eq!(
        extract_provider_session_id(&out),
        Some("claude-sid-1".to_string())
    );
}

#[test]
fn extract_provider_session_id_returns_none_when_absent() {
    let out = empty_output("codex");
    assert_eq!(extract_provider_session_id(&out), None);
}

#[test]
fn find_usage_limit_pulls_text_from_assistant_messages_too() {
    let cfg = UsageLimitConfig::default();
    let mut out = empty_output("claude");
    out.events.push(Event::AssistantMessage {
        content: vec![ContentBlock::Text {
            text: "Claude AI usage limit reached|1893456000".to_string(),
        }],
        usage: None,
        parent_tool_use_id: None,
    });

    let hit = find_usage_limit_in_output(&out, "claude", &cfg).expect("should detect");
    assert_eq!(hit.reset_at.unwrap().timestamp(), 1_893_456_000);
}
