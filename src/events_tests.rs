use super::*;

#[test]
fn event_type_name_covers_all_variants() {
    let kind = LogEventKind::SessionStarted {
        command: "exec".into(),
        model: None,
        cwd: None,
        resumed: false,
        backfilled: false,
    };
    assert_eq!(event_type_name(&kind), "session_started");

    let kind = LogEventKind::ToolCall {
        tool_name: "Bash".into(),
        tool_kind: None,
        tool_id: None,
        input: None,
    };
    assert_eq!(event_type_name(&kind), "tool_call");

    let kind = LogEventKind::SessionEnded {
        success: true,
        error: None,
    };
    assert_eq!(event_type_name(&kind), "session_ended");
}
