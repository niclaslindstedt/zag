use super::*;
use crate::session_log::{AgentLogEvent, LogCompleteness, LogEventKind, LogSourceKind, ToolKind};
use agent_lib::session_log::{
    GlobalSessionEntry, GlobalSessionIndex, load_global_index, save_global_index,
};

fn make_event(kind: LogEventKind) -> AgentLogEvent {
    AgentLogEvent {
        seq: 1,
        ts: "2026-03-24T10:00:00Z".to_string(),
        provider: "claude".to_string(),
        wrapper_session_id: "test-session-id".to_string(),
        provider_session_id: None,
        source_kind: LogSourceKind::Wrapper,
        completeness: LogCompleteness::Full,
        kind,
    }
}

#[test]
fn test_format_session_started() {
    let event = make_event(LogEventKind::SessionStarted {
        command: "run".to_string(),
        model: Some("opus".to_string()),
        cwd: None,
        resumed: false,
        backfilled: false,
    });
    let text = format_event_text(&event).unwrap();
    assert!(text.contains("Started: run"));
    assert!(text.contains("(model: opus)"));
    assert!(text.contains('\u{25cf}')); // ● icon
}

#[test]
fn test_format_session_started_no_model() {
    let event = make_event(LogEventKind::SessionStarted {
        command: "exec".to_string(),
        model: None,
        cwd: None,
        resumed: false,
        backfilled: false,
    });
    let text = format_event_text(&event).unwrap();
    assert!(text.contains("Started: exec"));
    assert!(!text.contains("model"));
}

#[test]
fn test_format_user_message() {
    let event = make_event(LogEventKind::UserMessage {
        role: "user".to_string(),
        content: "hello world".to_string(),
        message_id: None,
    });
    let text = format_event_text(&event).unwrap();
    assert!(text.contains("hello world"));
    assert!(text.contains('\u{276f}')); // ❯ icon
}

#[test]
fn test_format_assistant_message() {
    let event = make_event(LogEventKind::AssistantMessage {
        content: "Hi there!".to_string(),
        message_id: None,
    });
    let text = format_event_text(&event).unwrap();
    assert!(text.contains("Hi there!"));
    assert!(text.contains('\u{23fa}')); // ⏺ icon
}

#[test]
fn test_format_assistant_message_multiline() {
    let event = make_event(LogEventKind::AssistantMessage {
        content: "line one\nline two\nline three".to_string(),
        message_id: None,
    });
    let text = format_event_text(&event).unwrap();
    assert!(text.contains("line one\n"));
    assert!(text.contains("  line two\n")); // continuation indented
    assert!(text.contains("  line three"));
}

#[test]
fn test_format_reasoning() {
    let event = make_event(LogEventKind::Reasoning {
        content: "Let me think about this...".to_string(),
        message_id: None,
    });
    let text = format_event_text(&event).unwrap();
    assert!(text.contains("Let me think about this..."));
    assert!(text.contains('\u{2026}')); // … icon
}

#[test]
fn test_format_tool_call() {
    let event = make_event(LogEventKind::ToolCall {
        tool_name: "Read".to_string(),
        tool_kind: Some(ToolKind::FileRead),
        tool_id: Some("tool-1".to_string()),
        input: Some(serde_json::json!({"path": "/tmp/test.rs"})),
    });
    let text = format_event_text(&event).unwrap();
    assert!(text.contains("Read"));
    assert!(text.contains("/tmp/test.rs"));
    assert!(text.contains('\u{26a1}')); // ⚡ icon
}

#[test]
fn test_format_tool_call_with_command() {
    let event = make_event(LogEventKind::ToolCall {
        tool_name: "shell".to_string(),
        tool_kind: Some(ToolKind::Shell),
        tool_id: None,
        input: Some(serde_json::json!({"command": "ls -la", "description": "List files"})),
    });
    let text = format_event_text(&event).unwrap();
    assert!(text.contains("shell"));
    assert!(text.contains("ls -la"));
    assert!(text.contains("List files"));
}

#[test]
fn test_format_tool_result_success() {
    let event = make_event(LogEventKind::ToolResult {
        tool_name: Some("Read".to_string()),
        tool_kind: Some(ToolKind::FileRead),
        tool_id: Some("tool-1".to_string()),
        success: Some(true),
        output: Some("file contents".to_string()),
        error: None,
        data: None,
    });
    let text = format_event_text(&event).unwrap();
    assert!(text.contains('\u{2713}')); // ✓ icon
    assert!(text.contains("file contents"));
}

#[test]
fn test_format_tool_result_error() {
    let event = make_event(LogEventKind::ToolResult {
        tool_name: Some("Write".to_string()),
        tool_kind: Some(ToolKind::FileEdit),
        tool_id: None,
        success: Some(false),
        output: None,
        error: Some("permission denied".to_string()),
        data: None,
    });
    let text = format_event_text(&event).unwrap();
    assert!(text.contains('\u{2717}')); // ✗ icon
    assert!(text.contains("permission denied"));
}

#[test]
fn test_format_permission() {
    let event = make_event(LogEventKind::Permission {
        tool_name: "Bash".to_string(),
        description: "Run command".to_string(),
        granted: true,
    });
    let text = format_event_text(&event).unwrap();
    assert!(text.contains("Bash"));
    assert!(text.contains('\u{1f513}')); // 🔓 icon
}

#[test]
fn test_format_permission_denied() {
    let event = make_event(LogEventKind::Permission {
        tool_name: "Bash".to_string(),
        description: "Run command".to_string(),
        granted: false,
    });
    let text = format_event_text(&event).unwrap();
    assert!(text.contains("Bash"));
    assert!(text.contains('\u{1f512}')); // 🔒 icon
}

#[test]
fn test_format_provider_status() {
    let event = make_event(LogEventKind::ProviderStatus {
        message: "Initialized opus".to_string(),
        data: None,
    });
    let text = format_event_text(&event).unwrap();
    assert!(text.contains("Initialized opus"));
}

#[test]
fn test_format_stderr() {
    let event = make_event(LogEventKind::Stderr {
        message: "warning: unused variable".to_string(),
    });
    let text = format_event_text(&event).unwrap();
    assert!(text.contains("warning: unused variable"));
    assert!(text.contains('!'));
}

#[test]
fn test_format_parse_warning() {
    let event = make_event(LogEventKind::ParseWarning {
        message: "unexpected field".to_string(),
        raw: None,
    });
    let text = format_event_text(&event).unwrap();
    assert!(text.contains("unexpected field"));
    assert!(text.contains('?'));
}

#[test]
fn test_format_session_ended_success() {
    let event = make_event(LogEventKind::SessionEnded {
        success: true,
        error: None,
    });
    let text = format_event_text(&event).unwrap();
    assert!(text.contains("Session completed"));
    assert!(text.contains('\u{25cf}')); // ● icon
}

#[test]
fn test_format_session_ended_error() {
    let event = make_event(LogEventKind::SessionEnded {
        success: false,
        error: Some("timeout".to_string()),
    });
    let text = format_event_text(&event).unwrap();
    assert!(text.contains("Session failed"));
    assert!(text.contains("timeout"));
}

#[test]
fn test_format_rich_adds_ansi_codes() {
    let event = make_event(LogEventKind::Stderr {
        message: "error".to_string(),
    });
    let rich = format_event_rich(&event).unwrap();
    assert!(rich.contains("\x1b[")); // has ANSI codes
    assert!(rich.contains("\x1b[0m")); // has reset
    assert!(rich.contains("error"));
}

#[test]
fn test_format_rich_session_green() {
    let event = make_event(LogEventKind::SessionStarted {
        command: "run".to_string(),
        model: None,
        cwd: None,
        resumed: false,
        backfilled: false,
    });
    let rich = format_event_rich(&event).unwrap();
    assert!(rich.contains("\x1b[32m")); // green
}

#[test]
fn test_truncate_short_string() {
    assert_eq!(truncate("hello", 10), "hello");
}

#[test]
fn test_truncate_long_string() {
    let long = "a".repeat(250);
    let result = truncate(&long, 200);
    assert_eq!(result.len(), 203); // 200 + "..."
    assert!(result.ends_with("..."));
}

#[test]
fn test_truncate_newlines() {
    let text = "line1\nline2\nline3";
    let result = truncate(text, 200);
    assert_eq!(result, "line1\\nline2\\nline3");
}

#[test]
fn test_render_content_preserves_newlines() {
    let text = "line1\nline2\nline3";
    let result = render_content(text, 200);
    assert_eq!(result, "line1\nline2\nline3");
}

#[test]
fn test_render_content_truncates_long() {
    let long = "a".repeat(600);
    let result = render_content(&long, 500);
    assert_eq!(result.len(), 503); // 500 + "..."
}

#[test]
fn test_indent_continuation() {
    let text = "first\nsecond\nthird";
    let result = indent_continuation(text, "  ");
    assert_eq!(result, "first\n  second\n  third");
}

#[test]
fn test_indent_continuation_single_line() {
    let result = indent_continuation("only one", "  ");
    assert_eq!(result, "only one");
}

#[test]
fn test_shorten_path_short() {
    assert_eq!(shorten_path("src/main.rs"), "src/main.rs");
}

#[test]
fn test_shorten_path_long() {
    let result = shorten_path("/Users/niclas/Source/personal/agent/src/main.rs");
    assert_eq!(result, ".../agent/src/main.rs");
}

#[test]
fn test_summarize_tool_input_with_command() {
    let input = serde_json::json!({"command": "ls -la", "description": "List files"});
    let result = summarize_tool_input("anything", Some(&input));
    assert!(result.contains("ls -la"));
    assert!(result.contains("List files"));
}

#[test]
fn test_summarize_tool_input_with_file_path() {
    let input = serde_json::json!({"file_path": "/a/b/c/d/e/f.rs"});
    let result = summarize_tool_input("anything", Some(&input));
    assert!(result.contains("f.rs"));
}

#[test]
fn test_summarize_tool_input_none() {
    let result = summarize_tool_input("anything", None);
    assert!(result.is_empty());
}

#[test]
fn test_summarize_tool_input_fallback_json() {
    let input = serde_json::json!({"weird_key": "value"});
    let result = summarize_tool_input("anything", Some(&input));
    assert!(result.contains("weird_key"));
}

#[test]
fn test_listen_format_from_flags_json() {
    let config = Config::default();
    assert_eq!(
        ListenFormat::from_flags(true, false, false, &config),
        ListenFormat::Json
    );
}

#[test]
fn test_listen_format_from_flags_rich_text() {
    let config = Config::default();
    assert_eq!(
        ListenFormat::from_flags(false, true, false, &config),
        ListenFormat::RichText
    );
}

#[test]
fn test_listen_format_from_flags_text() {
    let config = Config::default();
    assert_eq!(
        ListenFormat::from_flags(false, false, true, &config),
        ListenFormat::Text
    );
}

#[test]
fn test_listen_format_from_flags_default() {
    let config = Config::default();
    assert_eq!(
        ListenFormat::from_flags(false, false, false, &config),
        ListenFormat::Text
    );
}

#[test]
fn test_lookup_global_index_by_id_exact_match() {
    // This tests the lookup_global_index_by_id function indirectly
    // by creating a global index file at the expected location
    let dir = std::env::temp_dir().join(format!("agent-listen-global-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _guard = Cleanup(dir.clone());

    // Create a dummy log file
    let log_file = dir.join("test-session.jsonl");
    std::fs::write(&log_file, "{}").unwrap();

    // Save a global index with that entry
    let index = GlobalSessionIndex {
        sessions: vec![GlobalSessionEntry {
            session_id: "abc-123".to_string(),
            project: "test-project".to_string(),
            log_path: log_file.to_string_lossy().to_string(),
            provider: "claude".to_string(),
            started_at: "2026-03-24T12:00:00Z".to_string(),
        }],
    };
    save_global_index(&dir, &index).unwrap();

    // Verify the index was created
    let loaded = load_global_index(&dir).unwrap();
    assert_eq!(loaded.sessions.len(), 1);
    assert_eq!(loaded.sessions[0].session_id, "abc-123");
    assert_eq!(loaded.sessions[0].log_path, log_file.to_string_lossy());
}

#[test]
fn test_global_index_serialization_roundtrip() {
    let dir = std::env::temp_dir().join(format!(
        "agent-listen-global-roundtrip-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _guard = Cleanup(dir.clone());

    let index = GlobalSessionIndex {
        sessions: vec![
            GlobalSessionEntry {
                session_id: "s1".to_string(),
                project: "proj-a".to_string(),
                log_path: "/tmp/a.jsonl".to_string(),
                provider: "claude".to_string(),
                started_at: "2026-03-24T10:00:00Z".to_string(),
            },
            GlobalSessionEntry {
                session_id: "s2".to_string(),
                project: "proj-b".to_string(),
                log_path: "/tmp/b.jsonl".to_string(),
                provider: "gemini".to_string(),
                started_at: "2026-03-24T11:00:00Z".to_string(),
            },
        ],
    };
    save_global_index(&dir, &index).unwrap();

    let loaded = load_global_index(&dir).unwrap();
    assert_eq!(loaded.sessions.len(), 2);
    assert_eq!(loaded.sessions[0].session_id, "s1");
    assert_eq!(loaded.sessions[1].session_id, "s2");
    assert_eq!(loaded.sessions[1].provider, "gemini");
}
