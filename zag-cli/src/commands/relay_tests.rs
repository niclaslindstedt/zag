use super::*;

#[test]
fn test_record_event_assistant_message() {
    let dir = tempfile::tempdir().unwrap();
    let logs_dir = dir.path().to_path_buf();
    let writer = SessionLogWriter::create(
        &logs_dir,
        SessionLogMetadata {
            provider: "claude".to_string(),
            wrapper_session_id: "test-session".to_string(),
            provider_session_id: None,
            workspace_path: None,
            command: "interactive".to_string(),
            model: Some("opus".to_string()),
            resumed: false,
            backfilled: false,
        },
    )
    .unwrap();

    let event = Event::AssistantMessage {
        content: vec![ContentBlock::Text {
            text: "Hello world".to_string(),
        }],
        usage: None,
        parent_tool_use_id: None,
    };

    record_event(&writer, &event).unwrap();

    let log_path = writer.log_path().unwrap();
    let content = std::fs::read_to_string(log_path).unwrap();
    assert!(content.contains("assistant_message"));
    assert!(content.contains("Hello world"));
}

#[test]
fn test_record_event_user_message() {
    let dir = tempfile::tempdir().unwrap();
    let logs_dir = dir.path().to_path_buf();
    let writer = SessionLogWriter::create(
        &logs_dir,
        SessionLogMetadata {
            provider: "claude".to_string(),
            wrapper_session_id: "test-session-2".to_string(),
            provider_session_id: None,
            workspace_path: None,
            command: "interactive".to_string(),
            model: None,
            resumed: false,
            backfilled: false,
        },
    )
    .unwrap();

    let event = Event::UserMessage {
        content: vec![ContentBlock::Text {
            text: "Test input".to_string(),
        }],
    };

    record_event(&writer, &event).unwrap();

    let log_path = writer.log_path().unwrap();
    let content = std::fs::read_to_string(log_path).unwrap();
    assert!(content.contains("user_message"));
    assert!(content.contains("Test input"));
}
