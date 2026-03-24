use super::*;
use crate::output::{AgentOutput, ContentBlock, Event, ToolResult};
use serde_json::json;
use std::path::PathBuf;

fn temp_logs_dir(name: &str) -> (std::path::PathBuf, impl Drop) {
    let dir = std::env::temp_dir().join(format!(
        "agent-lib-session-log-test-{}-{}",
        std::process::id(),
        name
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let logs = dir.join("logs");
    std::fs::create_dir_all(&logs).unwrap();

    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    (logs, Cleanup(dir))
}

struct DummyBackfillAdapter;

impl HistoricalLogAdapter for DummyBackfillAdapter {
    fn backfill(&self, _root: Option<&str>) -> Result<Vec<BackfilledSession>> {
        Ok(vec![BackfilledSession {
            metadata: SessionLogMetadata {
                provider: "dummy".to_string(),
                wrapper_session_id: "dummy-1".to_string(),
                provider_session_id: Some("native-1".to_string()),
                workspace_path: None,
                command: "backfill".to_string(),
                model: None,
                resumed: false,
                backfilled: true,
            },
            completeness: LogCompleteness::MetadataOnly,
            source_paths: vec!["/tmp/provider.log".to_string()],
            events: vec![(
                LogSourceKind::Backfill,
                LogEventKind::ProviderStatus {
                    message: "backfilled".to_string(),
                    data: None,
                },
            )],
        }])
    }
}

#[test]
fn test_writer_emits_events_and_updates_index() {
    let (logs_dir, _guard) = temp_logs_dir("writer");
    let metadata = SessionLogMetadata {
        provider: "claude".to_string(),
        wrapper_session_id: "session-1".to_string(),
        provider_session_id: None,
        workspace_path: Some("/tmp/workspace".to_string()),
        command: "run".to_string(),
        model: Some("opus".to_string()),
        resumed: false,
        backfilled: false,
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let coordinator = SessionLogCoordinator::start(&logs_dir, metadata, None).unwrap();
        record_prompt(coordinator.writer(), Some("hello")).unwrap();
        coordinator
            .writer()
            .emit(
                LogSourceKind::Wrapper,
                LogEventKind::AssistantMessage {
                    content: "world".to_string(),
                    message_id: Some("msg-1".to_string()),
                },
            )
            .unwrap();
        let log_path = coordinator.writer().log_path().unwrap();
        coordinator.finish(true, None).await.unwrap();

        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("\"session_started\""));
        assert!(content.contains("\"user_message\""));
        assert!(content.contains("\"assistant_message\""));
        assert!(content.contains("\"session_ended\""));
    });

    let index_path = logs_dir.join("index.json");
    let index: SessionLogIndex =
        serde_json::from_str(&std::fs::read_to_string(index_path).unwrap()).unwrap();
    assert_eq!(index.sessions.len(), 1);
    assert_eq!(index.sessions[0].wrapper_session_id, "session-1");
}

#[test]
fn test_record_agent_output_maps_core_events() {
    let (logs_dir, _guard) = temp_logs_dir("agent-output");
    let writer = SessionLogWriter::create(
        &logs_dir,
        SessionLogMetadata {
            provider: "codex".to_string(),
            wrapper_session_id: "session-2".to_string(),
            provider_session_id: None,
            workspace_path: None,
            command: "exec".to_string(),
            model: Some("gpt-5.4".to_string()),
            resumed: false,
            backfilled: false,
        },
    )
    .unwrap();

    let output = AgentOutput {
        agent: "codex".to_string(),
        session_id: "native-2".to_string(),
        events: vec![
            Event::AssistantMessage {
                content: vec![
                    ContentBlock::Text {
                        text: "answer".to_string(),
                    },
                    ContentBlock::ToolUse {
                        id: "tool-1".to_string(),
                        name: "exec_command".to_string(),
                        input: json!({"cmd":"pwd"}),
                    },
                ],
                usage: None,
            },
            Event::ToolExecution {
                tool_name: "exec_command".to_string(),
                tool_id: "tool-1".to_string(),
                input: json!({"cmd":"pwd"}),
                result: ToolResult {
                    success: true,
                    output: Some("/tmp".to_string()),
                    error: None,
                    data: None,
                },
            },
        ],
        result: Some("answer".to_string()),
        is_error: false,
        total_cost_usd: None,
        usage: None,
    };

    record_agent_output(&writer, &output).unwrap();
    let content = std::fs::read_to_string(writer.log_path().unwrap()).unwrap();
    assert!(content.contains("\"assistant_message\""));
    assert!(content.contains("\"tool_call\""));
    assert!(content.contains("\"tool_result\""));
    assert!(content.contains("native-2"));
}

#[test]
fn test_run_backfill_is_idempotent() {
    let (logs_dir, _guard) = temp_logs_dir("backfill");
    let adapter = DummyBackfillAdapter;
    run_backfill(&logs_dir, None, &[&adapter]).unwrap();
    run_backfill(&logs_dir, None, &[&adapter]).unwrap();

    let index_path = logs_dir.join("index.json");
    let index: SessionLogIndex =
        serde_json::from_str(&std::fs::read_to_string(index_path).unwrap()).unwrap();
    assert_eq!(index.sessions.len(), 1);
}

#[test]
fn test_global_index_upsert_and_load() {
    let dir = std::env::temp_dir().join(format!(
        "agent-lib-global-index-test-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _guard = Cleanup(dir.clone());

    // Empty index initially
    let index = load_global_index(&dir).unwrap();
    assert!(index.sessions.is_empty());

    // Insert an entry
    upsert_global_entry(
        &dir,
        GlobalSessionEntry {
            session_id: "session-1".to_string(),
            project: "test-project".to_string(),
            log_path: "/tmp/logs/session-1.jsonl".to_string(),
            provider: "claude".to_string(),
            started_at: "2026-03-24T12:00:00Z".to_string(),
        },
    )
    .unwrap();

    let index = load_global_index(&dir).unwrap();
    assert_eq!(index.sessions.len(), 1);
    assert_eq!(index.sessions[0].session_id, "session-1");

    // Upsert same ID updates fields
    upsert_global_entry(
        &dir,
        GlobalSessionEntry {
            session_id: "session-1".to_string(),
            project: "test-project".to_string(),
            log_path: "/tmp/logs/session-1-updated.jsonl".to_string(),
            provider: "gemini".to_string(),
            started_at: "2026-03-24T13:00:00Z".to_string(),
        },
    )
    .unwrap();

    let index = load_global_index(&dir).unwrap();
    assert_eq!(index.sessions.len(), 1);
    assert_eq!(
        index.sessions[0].log_path,
        "/tmp/logs/session-1-updated.jsonl"
    );
    assert_eq!(index.sessions[0].provider, "gemini");

    // Insert a second entry
    upsert_global_entry(
        &dir,
        GlobalSessionEntry {
            session_id: "session-2".to_string(),
            project: "other-project".to_string(),
            log_path: "/tmp/logs/session-2.jsonl".to_string(),
            provider: "codex".to_string(),
            started_at: "2026-03-24T14:00:00Z".to_string(),
        },
    )
    .unwrap();

    let index = load_global_index(&dir).unwrap();
    assert_eq!(index.sessions.len(), 2);
}

#[test]
fn test_writer_populates_global_index_when_configured() {
    let (logs_dir, _guard) = temp_logs_dir("global-writer");

    let global_dir = std::env::temp_dir().join(format!(
        "agent-lib-global-writer-test-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&global_dir);
    std::fs::create_dir_all(&global_dir).unwrap();

    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _gguard = Cleanup(global_dir.clone());

    let writer = SessionLogWriter::create(
        &logs_dir,
        SessionLogMetadata {
            provider: "claude".to_string(),
            wrapper_session_id: "global-test-1".to_string(),
            provider_session_id: None,
            workspace_path: None,
            command: "run".to_string(),
            model: Some("opus".to_string()),
            resumed: false,
            backfilled: false,
        },
    )
    .unwrap();

    // Before setting global dir, global index should be empty
    let index = load_global_index(&global_dir).unwrap();
    assert!(index.sessions.is_empty());

    // Set global dir and trigger an upsert via set_provider_session_id
    writer.set_global_index_dir(global_dir.clone()).unwrap();
    writer
        .set_provider_session_id(Some("native-1".to_string()))
        .unwrap();

    let index = load_global_index(&global_dir).unwrap();
    assert_eq!(index.sessions.len(), 1);
    assert_eq!(index.sessions[0].session_id, "global-test-1");
    assert_eq!(index.sessions[0].provider, "claude");
}
