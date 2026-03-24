use super::*;
use crate::session_log::{AgentLogEvent, LogCompleteness, LogEventKind, LogSourceKind};

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
    assert_eq!(text, "[session] Started: run (model: opus)");
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
    assert_eq!(text, "[session] Started: exec");
}

#[test]
fn test_format_user_message() {
    let event = make_event(LogEventKind::UserMessage {
        role: "user".to_string(),
        content: "hello world".to_string(),
        message_id: None,
    });
    let text = format_event_text(&event).unwrap();
    assert_eq!(text, "[user] hello world");
}

#[test]
fn test_format_assistant_message() {
    let event = make_event(LogEventKind::AssistantMessage {
        content: "Hi there!".to_string(),
        message_id: None,
    });
    let text = format_event_text(&event).unwrap();
    assert_eq!(text, "[assistant] Hi there!");
}

#[test]
fn test_format_reasoning() {
    let event = make_event(LogEventKind::Reasoning {
        content: "Let me think about this...".to_string(),
        message_id: None,
    });
    let text = format_event_text(&event).unwrap();
    assert_eq!(text, "[thinking] Let me think about this...");
}

#[test]
fn test_format_tool_call() {
    let event = make_event(LogEventKind::ToolCall {
        tool_name: "Read".to_string(),
        tool_id: Some("tool-1".to_string()),
        input: Some(serde_json::json!({"path": "/tmp/test.rs"})),
    });
    let text = format_event_text(&event).unwrap();
    assert!(text.starts_with("[tool] Read("));
    assert!(text.contains("/tmp/test.rs"));
}

#[test]
fn test_format_tool_result_success() {
    let event = make_event(LogEventKind::ToolResult {
        tool_name: Some("Read".to_string()),
        tool_id: Some("tool-1".to_string()),
        success: Some(true),
        output: Some("file contents".to_string()),
        error: None,
        data: None,
    });
    let text = format_event_text(&event).unwrap();
    assert_eq!(text, "[result] Read: success: file contents");
}

#[test]
fn test_format_tool_result_error() {
    let event = make_event(LogEventKind::ToolResult {
        tool_name: Some("Write".to_string()),
        tool_id: None,
        success: Some(false),
        output: None,
        error: Some("permission denied".to_string()),
        data: None,
    });
    let text = format_event_text(&event).unwrap();
    assert_eq!(text, "[result] Write: error: permission denied");
}

#[test]
fn test_format_permission() {
    let event = make_event(LogEventKind::Permission {
        tool_name: "Bash".to_string(),
        description: "Run command".to_string(),
        granted: true,
    });
    let text = format_event_text(&event).unwrap();
    assert_eq!(text, "[permission] Bash: granted");
}

#[test]
fn test_format_permission_denied() {
    let event = make_event(LogEventKind::Permission {
        tool_name: "Bash".to_string(),
        description: "Run command".to_string(),
        granted: false,
    });
    let text = format_event_text(&event).unwrap();
    assert_eq!(text, "[permission] Bash: denied");
}

#[test]
fn test_format_provider_status() {
    let event = make_event(LogEventKind::ProviderStatus {
        message: "Initialized opus".to_string(),
        data: None,
    });
    let text = format_event_text(&event).unwrap();
    assert_eq!(text, "[status] Initialized opus");
}

#[test]
fn test_format_stderr() {
    let event = make_event(LogEventKind::Stderr {
        message: "warning: unused variable".to_string(),
    });
    let text = format_event_text(&event).unwrap();
    assert_eq!(text, "[stderr] warning: unused variable");
}

#[test]
fn test_format_parse_warning() {
    let event = make_event(LogEventKind::ParseWarning {
        message: "unexpected field".to_string(),
        raw: None,
    });
    let text = format_event_text(&event).unwrap();
    assert_eq!(text, "[warning] unexpected field");
}

#[test]
fn test_format_session_ended_success() {
    let event = make_event(LogEventKind::SessionEnded {
        success: true,
        error: None,
    });
    let text = format_event_text(&event).unwrap();
    assert_eq!(text, "[session] Ended (success: true)");
}

#[test]
fn test_format_session_ended_error() {
    let event = make_event(LogEventKind::SessionEnded {
        success: false,
        error: Some("timeout".to_string()),
    });
    let text = format_event_text(&event).unwrap();
    assert_eq!(text, "[session] Ended (success: false) (timeout)");
}

#[test]
fn test_format_colored_adds_ansi_codes() {
    let event = make_event(LogEventKind::Stderr {
        message: "error".to_string(),
    });
    let colored = format_event_colored(&event).unwrap();
    assert!(colored.starts_with("\x1b[31m")); // red
    assert!(colored.ends_with("\x1b[0m")); // reset
    assert!(colored.contains("[stderr] error"));
}

#[test]
fn test_format_colored_session_green() {
    let event = make_event(LogEventKind::SessionStarted {
        command: "run".to_string(),
        model: None,
        cwd: None,
        resumed: false,
        backfilled: false,
    });
    let colored = format_event_colored(&event).unwrap();
    assert!(colored.starts_with("\x1b[32m")); // green
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
fn test_listen_format_from_flags_json() {
    let config = Config::default();
    assert_eq!(
        ListenFormat::from_flags(true, false, false, &config),
        ListenFormat::Json
    );
}

#[test]
fn test_listen_format_from_flags_colors() {
    let config = Config::default();
    assert_eq!(
        ListenFormat::from_flags(false, true, false, &config),
        ListenFormat::ColoredText
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
