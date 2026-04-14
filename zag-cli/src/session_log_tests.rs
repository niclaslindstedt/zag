use super::*;
use crate::output::{AgentOutput, ContentBlock, Event, ToolResult};
use serde_json::json;

fn temp_logs(name: &str) -> (std::path::PathBuf, impl Drop) {
    let dir = std::env::temp_dir().join(format!(
        "zag-session-log-test-{}-{}",
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
    let (logs_dir, _guard) = temp_logs("writer");
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
    let (logs_dir, _guard) = temp_logs("zag-output");
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
                parent_tool_use_id: None,
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
                parent_tool_use_id: None,
            },
        ],
        result: Some("answer".to_string()),
        is_error: false,
        exit_code: None,
        error_message: None,
        total_cost_usd: None,
        usage: None,
        model: None,
        provider: Some("codex".to_string()),
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
    let (logs_dir, _guard) = temp_logs("backfill");
    let adapter = DummyBackfillAdapter;
    run_backfill(&logs_dir, None, &[&adapter]).unwrap();
    run_backfill(&logs_dir, None, &[&adapter]).unwrap();

    let index_path = logs_dir.join("index.json");
    let index: SessionLogIndex =
        serde_json::from_str(&std::fs::read_to_string(index_path).unwrap()).unwrap();
    assert_eq!(index.sessions.len(), 1);
}
