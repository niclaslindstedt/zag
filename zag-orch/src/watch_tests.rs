use super::*;
use zag_agent::session_log::{LogCompleteness, LogSourceKind};

fn make_event(kind: LogEventKind) -> AgentLogEvent {
    AgentLogEvent {
        seq: 1,
        ts: "2024-01-01T00:00:00Z".to_string(),
        provider: "claude".to_string(),
        wrapper_session_id: "test-session-123".to_string(),
        provider_session_id: None,
        source_kind: LogSourceKind::Wrapper,
        completeness: LogCompleteness::Full,
        kind,
    }
}

#[test]
fn expand_template_replaces_variables() {
    let event = make_event(LogEventKind::SessionEnded {
        success: true,
        error: None,
    });
    let result = expand_template("done: {session_id} by {provider}", &event);
    assert_eq!(result, "done: test-session-123 by claude");
}

#[test]
fn event_type_str_covers_variants() {
    assert_eq!(
        event_type_str(&LogEventKind::SessionEnded {
            success: true,
            error: None
        }),
        "session_ended"
    );
    assert_eq!(
        event_type_str(&LogEventKind::ToolCall {
            tool_name: "Bash".into(),
            tool_kind: None,
            tool_id: None,
            input: None,
        }),
        "tool_call"
    );
}

#[test]
fn matches_filter_success_true() {
    let event = make_event(LogEventKind::SessionEnded {
        success: true,
        error: None,
    });
    assert!(matches_filter(&event, "success=true"));
    assert!(!matches_filter(&event, "success=false"));
}

#[test]
fn matches_filter_tool_name() {
    let event = make_event(LogEventKind::ToolCall {
        tool_name: "Bash".into(),
        tool_kind: None,
        tool_id: None,
        input: None,
    });
    assert!(matches_filter(&event, "tool_name=bash"));
    assert!(!matches_filter(&event, "tool_name=read"));
}
