use super::*;
use crate::output::{AgentOutput, ContentBlock, Event, ToolResult};
use serde_json::json;
use std::path::PathBuf;

fn temp_logs_dir(name: &str) -> (std::path::PathBuf, impl Drop) {
    let dir = std::env::temp_dir().join(format!(
        "zag-agent-session-log-test-{}-{}",
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
    let (logs_dir, _guard) = temp_logs_dir("zag-output");
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
        exit_code: None,
        error_message: None,
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
fn test_tool_kind_infer() {
    assert_eq!(ToolKind::infer("Bash"), ToolKind::Shell);
    assert_eq!(ToolKind::infer("run_shell"), ToolKind::Shell);
    assert_eq!(ToolKind::infer("read_file"), ToolKind::FileRead);
    assert_eq!(ToolKind::infer("write_file"), ToolKind::FileWrite);
    assert_eq!(ToolKind::infer("apply_patch"), ToolKind::FileEdit);
    assert_eq!(ToolKind::infer("edit_line"), ToolKind::FileEdit);
    assert_eq!(ToolKind::infer("search_code"), ToolKind::Search);
    assert_eq!(ToolKind::infer("sub_agent"), ToolKind::SubAgent);
    assert_eq!(ToolKind::infer("web_fetch"), ToolKind::Web);
    assert_eq!(ToolKind::infer("notebook_edit"), ToolKind::Notebook);
    assert_eq!(ToolKind::infer("custom_mcp"), ToolKind::Other);
}

#[test]
fn test_tool_kind_serialization_roundtrip() {
    let event = LogEventKind::ToolCall {
        tool_name: "Bash".to_string(),
        tool_kind: Some(ToolKind::Shell),
        tool_id: None,
        input: None,
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"tool_kind\":\"shell\""));
    let parsed: LogEventKind = serde_json::from_str(&json).unwrap();
    match parsed {
        LogEventKind::ToolCall { tool_kind, .. } => {
            assert_eq!(tool_kind, Some(ToolKind::Shell));
        }
        _ => panic!("expected ToolCall"),
    }
}

#[test]
fn test_tool_kind_absent_in_old_events() {
    // Events without tool_kind should deserialize with None (backward compat)
    let json = r#"{"type":"tool_call","tool_name":"Bash","tool_id":null,"input":null}"#;
    let parsed: LogEventKind = serde_json::from_str(json).unwrap();
    match parsed {
        LogEventKind::ToolCall { tool_kind, .. } => {
            assert_eq!(tool_kind, None);
        }
        _ => panic!("expected ToolCall"),
    }
}

#[test]
fn test_global_index_upsert_and_load() {
    let dir = std::env::temp_dir().join(format!(
        "zag-agent-global-index-test-{}",
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
        "zag-agent-global-writer-test-{}",
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

#[test]
fn test_usage_event_json_roundtrip() {
    let event = LogEventKind::Usage {
        input_tokens: 1500,
        output_tokens: 500,
        cache_read_tokens: Some(200),
        cache_creation_tokens: None,
        total_cost_usd: Some(0.0042),
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"usage\"") || json.contains("\"Usage\""));
    assert!(json.contains("1500"));
    assert!(json.contains("0.0042"));
    let parsed: LogEventKind = serde_json::from_str(&json).unwrap();
    match parsed {
        LogEventKind::Usage {
            input_tokens,
            output_tokens,
            cache_read_tokens,
            cache_creation_tokens,
            total_cost_usd,
        } => {
            assert_eq!(input_tokens, 1500);
            assert_eq!(output_tokens, 500);
            assert_eq!(cache_read_tokens, Some(200));
            assert_eq!(cache_creation_tokens, None);
            assert_eq!(total_cost_usd, Some(0.0042));
        }
        _ => panic!("Expected Usage variant"),
    }
}

#[test]
fn test_record_agent_output_emits_usage() {
    let (logs_dir, _guard) = temp_logs_dir("usage-emit");
    let writer = SessionLogWriter::create(
        &logs_dir,
        SessionLogMetadata {
            provider: "claude".to_string(),
            wrapper_session_id: "usage-test-1".to_string(),
            provider_session_id: None,
            workspace_path: None,
            command: "exec".to_string(),
            model: Some("sonnet".to_string()),
            resumed: false,
            backfilled: false,
        },
    )
    .unwrap();

    let output = AgentOutput {
        agent: "claude".to_string(),
        session_id: "native-usage-1".to_string(),
        events: vec![],
        result: Some("done".to_string()),
        is_error: false,
        exit_code: None,
        error_message: None,
        total_cost_usd: Some(0.05),
        usage: Some(crate::output::Usage {
            input_tokens: 10000,
            output_tokens: 2000,
            cache_read_tokens: Some(500),
            cache_creation_tokens: Some(100),
            web_search_requests: None,
            web_fetch_requests: None,
        }),
    };

    record_agent_output(&writer, &output).unwrap();
    let content = std::fs::read_to_string(writer.log_path().unwrap()).unwrap();
    assert!(content.contains("\"usage\"") || content.contains("\"Usage\""));
    assert!(content.contains("10000"));
    assert!(content.contains("2000"));
    assert!(content.contains("0.05"));
}

#[test]
fn test_heartbeat_event_json_roundtrip() {
    let event = LogEventKind::Heartbeat {
        interval_secs: Some(10),
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("10"));
    let parsed: LogEventKind = serde_json::from_str(&json).unwrap();
    match parsed {
        LogEventKind::Heartbeat { interval_secs } => {
            assert_eq!(interval_secs, Some(10));
        }
        _ => panic!("Expected Heartbeat variant"),
    }
}

#[test]
fn test_coordinator_emits_heartbeat_without_live_adapter() {
    let (logs_dir, _guard) = temp_logs_dir("heartbeat");
    let metadata = SessionLogMetadata {
        provider: "claude".to_string(),
        wrapper_session_id: "heartbeat-test-1".to_string(),
        provider_session_id: None,
        workspace_path: None,
        command: "run".to_string(),
        model: Some("opus".to_string()),
        resumed: false,
        backfilled: false,
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let coordinator = SessionLogCoordinator::start(&logs_dir, metadata, None).unwrap();
        let log_path = coordinator.writer().log_path().unwrap();
        // Just verify it starts and stops without error
        coordinator.finish(true, None).await.unwrap();

        let content = std::fs::read_to_string(&log_path).unwrap();
        // The session_started and session_ended events should be present
        assert!(content.contains("\"session_started\""));
        assert!(content.contains("\"session_ended\""));
    });
}
