use super::*;
use crate::session_log::{LogEventKind, ToolKind};
use serde_json::json;
use std::collections::HashSet;
use std::io::Write as IoWrite;

// ===========================================================================
// tool_kind_from_name
// ===========================================================================

#[test]
fn test_tool_kind_bash() {
    assert_eq!(tool_kind_from_name("Bash"), ToolKind::Shell);
}

#[test]
fn test_tool_kind_read() {
    assert_eq!(tool_kind_from_name("Read"), ToolKind::FileRead);
}

#[test]
fn test_tool_kind_write() {
    assert_eq!(tool_kind_from_name("Write"), ToolKind::FileWrite);
}

#[test]
fn test_tool_kind_edit() {
    assert_eq!(tool_kind_from_name("Edit"), ToolKind::FileEdit);
}

#[test]
fn test_tool_kind_glob_grep() {
    assert_eq!(tool_kind_from_name("Glob"), ToolKind::Search);
    assert_eq!(tool_kind_from_name("Grep"), ToolKind::Search);
}

#[test]
fn test_tool_kind_agent() {
    assert_eq!(tool_kind_from_name("Agent"), ToolKind::SubAgent);
}

#[test]
fn test_tool_kind_web() {
    assert_eq!(tool_kind_from_name("WebFetch"), ToolKind::Web);
    assert_eq!(tool_kind_from_name("WebSearch"), ToolKind::Web);
}

#[test]
fn test_tool_kind_notebook() {
    assert_eq!(tool_kind_from_name("NotebookEdit"), ToolKind::Notebook);
}

#[test]
fn test_tool_kind_unknown() {
    assert_eq!(tool_kind_from_name("SomeCustomTool"), ToolKind::Other);
    assert_eq!(tool_kind_from_name(""), ToolKind::Other);
}

// ===========================================================================
// event_key
// ===========================================================================

#[test]
fn test_event_key_with_uuid() {
    let value = json!({
        "uuid": "abc-123",
        "type": "user",
        "timestamp": "2026-01-01T00:00:00Z",
    });
    let key = event_key(&value);
    assert_eq!(key, Some("abc-123".to_string()));
}

#[test]
fn test_event_key_without_uuid() {
    let value = json!({
        "type": "assistant",
        "timestamp": "2026-01-01T00:00:00Z",
        "sessionId": "sess-1",
    });
    let key = event_key(&value);
    assert_eq!(
        key,
        Some("2026-01-01T00:00:00Z:assistant:sess-1".to_string())
    );
}

#[test]
fn test_event_key_minimal() {
    let value = json!({});
    let key = event_key(&value);
    assert_eq!(key, Some("::".to_string()));
}

// ===========================================================================
// parse_claude_value
// ===========================================================================

#[test]
fn test_parse_user_message_string_content() {
    let value = json!({
        "uuid": "u1",
        "type": "user",
        "message": {
            "content": "hello world"
        }
    });
    let mut seen = HashSet::new();
    let events = parse_claude_value(&value, &mut seen);
    assert_eq!(events.len(), 1);
    match &events[0] {
        LogEventKind::UserMessage { content, role, .. } => {
            assert_eq!(content, "hello world");
            assert_eq!(role, "user");
        }
        other => panic!("Expected UserMessage, got {other:?}"),
    }
}

#[test]
fn test_parse_user_message_tool_result() {
    let value = json!({
        "uuid": "u2",
        "type": "user",
        "message": {
            "content": [
                {
                    "type": "tool_result",
                    "tool_use_id": "tool-1",
                    "is_error": false,
                    "content": "file contents here"
                }
            ]
        }
    });
    let mut seen = HashSet::new();
    let events = parse_claude_value(&value, &mut seen);
    assert_eq!(events.len(), 1);
    match &events[0] {
        LogEventKind::ToolResult {
            tool_id,
            success,
            output,
            ..
        } => {
            assert_eq!(tool_id.as_deref(), Some("tool-1"));
            assert_eq!(*success, Some(true)); // is_error=false → success=true
            assert_eq!(output.as_deref(), Some("file contents here"));
        }
        other => panic!("Expected ToolResult, got {other:?}"),
    }
}

#[test]
fn test_parse_assistant_text_block() {
    let value = json!({
        "uuid": "a1",
        "type": "assistant",
        "message": {
            "id": "msg-1",
            "content": [
                {"type": "text", "text": "Here is the answer"}
            ]
        }
    });
    let mut seen = HashSet::new();
    let events = parse_claude_value(&value, &mut seen);
    assert_eq!(events.len(), 1);
    match &events[0] {
        LogEventKind::AssistantMessage {
            content,
            message_id,
        } => {
            assert_eq!(content, "Here is the answer");
            assert_eq!(message_id.as_deref(), Some("msg-1"));
        }
        other => panic!("Expected AssistantMessage, got {other:?}"),
    }
}

#[test]
fn test_parse_assistant_thinking_block() {
    let value = json!({
        "uuid": "a2",
        "type": "assistant",
        "message": {
            "id": "msg-2",
            "content": [
                {"type": "thinking", "thinking": "Let me consider this..."}
            ]
        }
    });
    let mut seen = HashSet::new();
    let events = parse_claude_value(&value, &mut seen);
    assert_eq!(events.len(), 1);
    match &events[0] {
        LogEventKind::Reasoning { content, .. } => {
            assert_eq!(content, "Let me consider this...");
        }
        other => panic!("Expected Reasoning, got {other:?}"),
    }
}

#[test]
fn test_parse_assistant_tool_use_block() {
    let value = json!({
        "uuid": "a3",
        "type": "assistant",
        "message": {
            "content": [
                {
                    "type": "tool_use",
                    "id": "tool-abc",
                    "name": "Bash",
                    "input": {"command": "ls"}
                }
            ]
        }
    });
    let mut seen = HashSet::new();
    let events = parse_claude_value(&value, &mut seen);
    assert_eq!(events.len(), 1);
    match &events[0] {
        LogEventKind::ToolCall {
            tool_name,
            tool_kind,
            tool_id,
            input,
        } => {
            assert_eq!(tool_name, "Bash");
            assert_eq!(*tool_kind, Some(ToolKind::Shell));
            assert_eq!(tool_id.as_deref(), Some("tool-abc"));
            assert!(input.is_some());
        }
        other => panic!("Expected ToolCall, got {other:?}"),
    }
}

#[test]
fn test_parse_system_event() {
    let value = json!({
        "uuid": "s1",
        "type": "system",
        "data": "some system data"
    });
    let mut seen = HashSet::new();
    let events = parse_claude_value(&value, &mut seen);
    assert_eq!(events.len(), 1);
    assert!(matches!(&events[0], LogEventKind::ProviderStatus { .. }));
}

#[test]
fn test_parse_result_with_permission_denials() {
    let value = json!({
        "uuid": "r1",
        "type": "result",
        "result": "success",
        "permission_denials": [
            {"tool_name": "Bash", "tool_input": {"command": "rm -rf /"}}
        ]
    });
    let mut seen = HashSet::new();
    let events = parse_claude_value(&value, &mut seen);
    // Should produce 1 Permission event + 1 ProviderStatus event
    assert_eq!(events.len(), 2);
    let perm = &events[0];
    match perm {
        LogEventKind::Permission {
            tool_name, granted, ..
        } => {
            assert_eq!(tool_name, "Bash");
            assert!(!granted);
        }
        other => panic!("Expected Permission, got {other:?}"),
    }
    assert!(matches!(&events[1], LogEventKind::ProviderStatus { .. }));
}

#[test]
fn test_parse_queue_operation() {
    let value = json!({
        "uuid": "q1",
        "type": "queue-operation",
        "data": "queued"
    });
    let mut seen = HashSet::new();
    let events = parse_claude_value(&value, &mut seen);
    assert_eq!(events.len(), 1);
    assert!(matches!(&events[0], LogEventKind::ProviderStatus { .. }));
}

#[test]
fn test_parse_unknown_type() {
    let value = json!({
        "uuid": "x1",
        "type": "unknown_type",
    });
    let mut seen = HashSet::new();
    let events = parse_claude_value(&value, &mut seen);
    assert!(events.is_empty());
}

#[test]
fn test_parse_deduplication() {
    let value = json!({
        "uuid": "dup1",
        "type": "system",
    });
    let mut seen = HashSet::new();
    let events1 = parse_claude_value(&value, &mut seen);
    assert_eq!(events1.len(), 1);
    // Same value again — should be deduplicated
    let events2 = parse_claude_value(&value, &mut seen);
    assert!(events2.is_empty());
}

#[test]
fn test_parse_mixed_assistant_blocks() {
    let value = json!({
        "uuid": "mixed1",
        "type": "assistant",
        "message": {
            "id": "msg-mixed",
            "content": [
                {"type": "thinking", "thinking": "thinking..."},
                {"type": "text", "text": "Here is my answer"},
                {"type": "tool_use", "id": "t1", "name": "Read", "input": {"path": "/tmp"}}
            ]
        }
    });
    let mut seen = HashSet::new();
    let events = parse_claude_value(&value, &mut seen);
    assert_eq!(events.len(), 3);
    assert!(matches!(&events[0], LogEventKind::Reasoning { .. }));
    assert!(matches!(&events[1], LogEventKind::AssistantMessage { .. }));
    assert!(matches!(&events[2], LogEventKind::ToolCall { .. }));
}

// ===========================================================================
// backfill_session
// ===========================================================================

#[test]
fn test_backfill_session_valid() {
    let dir = std::env::temp_dir().join(format!("zag-claude-logs-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _guard = Cleanup(dir.clone());

    let path = dir.join("session.jsonl");
    let mut file = std::fs::File::create(&path).unwrap();
    writeln!(
        file,
        "{}",
        json!({
            "uuid": "line1",
            "type": "user",
            "sessionId": "native-sess-1",
            "cwd": "/home/user/project",
            "message": {"content": "hello"}
        })
    )
    .unwrap();
    writeln!(
        file,
        "{}",
        json!({
            "uuid": "line2",
            "type": "assistant",
            "sessionId": "native-sess-1",
            "message": {"content": [{"type": "text", "text": "response"}]}
        })
    )
    .unwrap();

    let result = backfill_session(&path).unwrap();
    assert!(result.is_some());
    let session = result.unwrap();
    assert_eq!(session.metadata.provider, "claude");
    assert_eq!(
        session.metadata.provider_session_id.as_deref(),
        Some("native-sess-1")
    );
    assert_eq!(
        session.metadata.workspace_path.as_deref(),
        Some("/home/user/project")
    );
    assert!(session.metadata.backfilled);
    assert_eq!(session.completeness, LogCompleteness::Full);
    assert!(!session.events.is_empty());
}

#[test]
fn test_backfill_session_no_session_id() {
    let dir =
        std::env::temp_dir().join(format!("zag-claude-logs-test-no-id-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _guard = Cleanup(dir.clone());

    let path = dir.join("no-id.jsonl");
    let mut file = std::fs::File::create(&path).unwrap();
    writeln!(
        file,
        "{}",
        json!({"type": "user", "message": {"content": "hello"}})
    )
    .unwrap();

    let result = backfill_session(&path).unwrap();
    assert!(result.is_none());
}

// ===========================================================================
// system_time_from_utc
// ===========================================================================

#[test]
fn test_system_time_from_utc_basic() {
    use chrono::TimeZone;
    let dt = chrono::Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap();
    let st = system_time_from_utc(dt);
    let expected_secs = dt.timestamp() as u64;
    let actual = st
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap();
    assert_eq!(actual.as_secs(), expected_secs);
}

// ===========================================================================
// file_contains_workspace
// ===========================================================================

#[test]
fn test_file_contains_workspace_match() {
    let dir = std::env::temp_dir().join(format!("zag-claude-logs-ws-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _guard = Cleanup(dir.clone());

    let path = dir.join("session.jsonl");
    let mut file = std::fs::File::create(&path).unwrap();
    writeln!(
        file,
        "{}",
        json!({"cwd": "/home/user/my-project", "type": "user"})
    )
    .unwrap();

    assert!(file_contains_workspace(&path, "/home/user/my-project"));
    assert!(!file_contains_workspace(&path, "/other/path"));
}

#[test]
fn test_file_contains_workspace_nonexistent() {
    let path = std::path::Path::new("/nonexistent/file.jsonl");
    assert!(!file_contains_workspace(path, "/anything"));
}
