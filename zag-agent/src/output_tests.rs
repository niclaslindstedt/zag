use super::*;
use std::collections::HashMap;

fn make_agent_output(events: Vec<Event>, result: Option<String>, is_error: bool) -> AgentOutput {
    AgentOutput {
        agent: "test".to_string(),
        session_id: "sess-1".to_string(),
        events,
        result,
        is_error,
        exit_code: None,
        error_message: None,
        total_cost_usd: None,
        usage: None,
    }
}

// --- AgentOutput::from_text ---

#[test]
fn test_from_text_basic() {
    let output = AgentOutput::from_text("codex", "hello world");
    assert_eq!(output.agent, "codex");
    assert_eq!(output.result, Some("hello world".to_string()));
    assert_eq!(output.final_result(), Some("hello world"));
    assert!(!output.is_error);
    assert!(output.session_id.is_empty());
    assert!(output.total_cost_usd.is_none());
    assert!(output.usage.is_none());
}

#[test]
fn test_from_text_creates_result_event() {
    let output = AgentOutput::from_text("gemini", "test");
    assert_eq!(output.events.len(), 1);
    if let Event::Result {
        success, message, ..
    } = &output.events[0]
    {
        assert!(success);
        assert_eq!(message.as_deref(), Some("test"));
    } else {
        panic!("Expected Result event");
    }
}

#[test]
fn test_from_text_empty_string() {
    let output = AgentOutput::from_text("test", "");
    assert_eq!(output.result, Some("".to_string()));
}

// --- AgentOutput methods ---

#[test]
fn test_final_result() {
    let output = make_agent_output(vec![], Some("hello".to_string()), false);
    assert_eq!(output.final_result(), Some("hello"));

    let output = make_agent_output(vec![], None, false);
    assert_eq!(output.final_result(), None);
}

#[test]
fn test_is_success() {
    assert!(make_agent_output(vec![], None, false).is_success());
    assert!(!make_agent_output(vec![], None, true).is_success());
}

#[test]
fn test_tool_executions() {
    let events = vec![
        Event::AssistantMessage {
            content: vec![ContentBlock::Text {
                text: "hi".to_string(),
            }],
            usage: None,
            parent_tool_use_id: None,
        },
        Event::ToolExecution {
            tool_name: "Bash".to_string(),
            tool_id: "t1".to_string(),
            input: serde_json::json!({}),
            result: ToolResult {
                success: true,
                output: Some("ok".to_string()),
                error: None,
                data: None,
            },
            parent_tool_use_id: None,
        },
        Event::ToolExecution {
            tool_name: "Read".to_string(),
            tool_id: "t2".to_string(),
            input: serde_json::json!({}),
            result: ToolResult {
                success: false,
                output: None,
                error: Some("err".to_string()),
                data: None,
            },
            parent_tool_use_id: None,
        },
    ];
    assert_eq!(
        make_agent_output(events, None, false)
            .tool_executions()
            .len(),
        2
    );
}

#[test]
fn test_errors() {
    let events = vec![
        Event::Error {
            message: "boom".to_string(),
            details: None,
        },
        Event::AssistantMessage {
            content: vec![],
            usage: None,
            parent_tool_use_id: None,
        },
    ];
    assert_eq!(make_agent_output(events, None, true).errors().len(), 1);
}

// --- to_log_entries ---

#[test]
fn test_to_log_entries_filters_by_level() {
    let events = vec![
        Event::Init {
            model: "test".to_string(),
            tools: vec![],
            working_directory: None,
            metadata: HashMap::new(),
        },
        Event::AssistantMessage {
            content: vec![ContentBlock::Text {
                text: "hello".to_string(),
            }],
            usage: None,
            parent_tool_use_id: None,
        },
    ];
    let output = make_agent_output(events, None, false);
    // Info level should include Init but not AssistantMessage (Debug level)
    assert_eq!(output.to_log_entries(LogLevel::Info).len(), 1);
    // Debug level should include both
    assert_eq!(output.to_log_entries(LogLevel::Debug).len(), 2);
}

// --- event_to_log_entry ---

#[test]
fn test_event_to_log_entry_init() {
    let event = Event::Init {
        model: "opus".to_string(),
        tools: vec![],
        working_directory: None,
        metadata: HashMap::new(),
    };
    let entry = event_to_log_entry(&event).unwrap();
    assert_eq!(entry.level, LogLevel::Info);
    assert!(entry.message.contains("opus"));
}

#[test]
fn test_event_to_log_entry_assistant_text() {
    let event = Event::AssistantMessage {
        content: vec![ContentBlock::Text {
            text: "hello".to_string(),
        }],
        usage: None,
        parent_tool_use_id: None,
    };
    let entry = event_to_log_entry(&event).unwrap();
    assert_eq!(entry.level, LogLevel::Debug);
    assert!(entry.message.contains("hello"));
}

#[test]
fn test_event_to_log_entry_tool_use_only_returns_none() {
    let event = Event::AssistantMessage {
        content: vec![ContentBlock::ToolUse {
            id: "id1".to_string(),
            name: "Bash".to_string(),
            input: serde_json::json!({}),
        }],
        usage: None,
        parent_tool_use_id: None,
    };
    assert!(event_to_log_entry(&event).is_none());
}

#[test]
fn test_event_to_log_entry_empty_content_returns_none() {
    let event = Event::AssistantMessage {
        content: vec![],
        usage: None,
        parent_tool_use_id: None,
    };
    assert!(event_to_log_entry(&event).is_none());
}

#[test]
fn test_event_to_log_entry_tool_execution_success() {
    let event = Event::ToolExecution {
        tool_name: "Bash".to_string(),
        tool_id: "id1".to_string(),
        input: serde_json::json!({}),
        result: ToolResult {
            success: true,
            output: Some("ok".to_string()),
            error: None,
            data: None,
        },
        parent_tool_use_id: None,
    };
    let entry = event_to_log_entry(&event).unwrap();
    assert_eq!(entry.level, LogLevel::Debug);
    assert!(entry.message.contains("successfully"));
}

#[test]
fn test_event_to_log_entry_tool_execution_failure() {
    let event = Event::ToolExecution {
        tool_name: "Bash".to_string(),
        tool_id: "id1".to_string(),
        input: serde_json::json!({}),
        result: ToolResult {
            success: false,
            output: None,
            error: Some("not found".to_string()),
            data: None,
        },
        parent_tool_use_id: None,
    };
    let entry = event_to_log_entry(&event).unwrap();
    assert_eq!(entry.level, LogLevel::Warn);
    assert!(entry.message.contains("not found"));
}

#[test]
fn test_event_to_log_entry_result_success() {
    let event = Event::Result {
        success: true,
        message: Some("done".to_string()),
        duration_ms: Some(1000),
        num_turns: Some(3),
    };
    let entry = event_to_log_entry(&event).unwrap();
    assert_eq!(entry.level, LogLevel::Info);
    assert_eq!(entry.message, "done");
}

#[test]
fn test_event_to_log_entry_result_failure_no_message() {
    let event = Event::Result {
        success: false,
        message: None,
        duration_ms: None,
        num_turns: None,
    };
    let entry = event_to_log_entry(&event).unwrap();
    assert_eq!(entry.level, LogLevel::Error);
    assert_eq!(entry.message, "Session failed");
}

#[test]
fn test_event_to_log_entry_result_success_no_message() {
    let event = Event::Result {
        success: true,
        message: None,
        duration_ms: None,
        num_turns: None,
    };
    let entry = event_to_log_entry(&event).unwrap();
    assert_eq!(entry.level, LogLevel::Info);
    assert_eq!(entry.message, "Session completed");
}

#[test]
fn test_event_to_log_entry_error() {
    let event = Event::Error {
        message: "broke".to_string(),
        details: Some(serde_json::json!({"code": 500})),
    };
    let entry = event_to_log_entry(&event).unwrap();
    assert_eq!(entry.level, LogLevel::Error);
    assert!(entry.data.is_some());
}

#[test]
fn test_event_to_log_entry_permission() {
    let granted = Event::PermissionRequest {
        tool_name: "Bash".to_string(),
        description: "run".to_string(),
        granted: true,
    };
    assert_eq!(event_to_log_entry(&granted).unwrap().level, LogLevel::Debug);

    let denied = Event::PermissionRequest {
        tool_name: "Bash".to_string(),
        description: "run".to_string(),
        granted: false,
    };
    assert_eq!(event_to_log_entry(&denied).unwrap().level, LogLevel::Warn);
}

#[test]
fn test_event_to_log_entry_turn_complete_with_reason() {
    let event = Event::TurnComplete {
        stop_reason: Some("end_turn".to_string()),
        turn_index: 2,
        usage: None,
    };
    let entry = event_to_log_entry(&event).unwrap();
    assert_eq!(entry.level, LogLevel::Debug);
    assert!(entry.message.contains("Turn 2"));
    assert!(entry.message.contains("end_turn"));
}

#[test]
fn test_event_to_log_entry_turn_complete_without_reason() {
    let event = Event::TurnComplete {
        stop_reason: None,
        turn_index: 0,
        usage: None,
    };
    let entry = event_to_log_entry(&event).unwrap();
    assert_eq!(entry.level, LogLevel::Debug);
    assert!(entry.message.contains("Turn 0"));
    assert!(entry.message.contains("none"));
}

// --- TurnComplete serde round-trip ---

#[test]
fn test_turn_complete_serializes_with_snake_case_type_tag() {
    let event = Event::TurnComplete {
        stop_reason: Some("tool_use".to_string()),
        turn_index: 3,
        usage: Some(Usage {
            input_tokens: 120,
            output_tokens: 45,
            cache_read_tokens: Some(10),
            cache_creation_tokens: Some(0),
            web_search_requests: None,
            web_fetch_requests: None,
        }),
    };
    let json = serde_json::to_value(&event).unwrap();
    assert_eq!(json["type"], "turn_complete");
    assert_eq!(json["stop_reason"], "tool_use");
    assert_eq!(json["turn_index"], 3);
    assert_eq!(json["usage"]["input_tokens"], 120);
    assert_eq!(json["usage"]["output_tokens"], 45);
}

#[test]
fn test_turn_complete_deserializes_from_json() {
    let json = r#"{
        "type": "turn_complete",
        "stop_reason": "end_turn",
        "turn_index": 0,
        "usage": null
    }"#;
    let event: Event = serde_json::from_str(json).unwrap();
    match event {
        Event::TurnComplete {
            stop_reason,
            turn_index,
            usage,
        } => {
            assert_eq!(stop_reason.as_deref(), Some("end_turn"));
            assert_eq!(turn_index, 0);
            assert!(usage.is_none());
        }
        other => panic!("expected TurnComplete, got {:?}", other),
    }
}

#[test]
fn test_turn_complete_round_trips_with_null_stop_reason() {
    let event = Event::TurnComplete {
        stop_reason: None,
        turn_index: 5,
        usage: None,
    };
    let json = serde_json::to_string(&event).unwrap();
    let parsed: Event = serde_json::from_str(&json).unwrap();
    match parsed {
        Event::TurnComplete {
            stop_reason,
            turn_index,
            ..
        } => {
            assert!(stop_reason.is_none());
            assert_eq!(turn_index, 5);
        }
        other => panic!("expected TurnComplete, got {:?}", other),
    }
}

// --- LogEntry Display ---

#[test]
fn test_log_entry_display() {
    let entry = LogEntry {
        level: LogLevel::Info,
        message: "test".to_string(),
        data: None,
        timestamp: None,
    };
    assert_eq!(format!("{}", entry), "[INFO] test");
}

#[test]
fn test_log_entry_display_all_levels() {
    for (level, prefix) in [
        (LogLevel::Debug, "[DEBUG]"),
        (LogLevel::Info, "[INFO]"),
        (LogLevel::Warn, "[WARN]"),
        (LogLevel::Error, "[ERROR]"),
    ] {
        let entry = LogEntry {
            level,
            message: "x".to_string(),
            data: None,
            timestamp: None,
        };
        assert!(format!("{}", entry).starts_with(prefix));
    }
}

// --- LogLevel ordering ---

#[test]
fn test_log_level_ordering() {
    assert!(LogLevel::Debug < LogLevel::Info);
    assert!(LogLevel::Info < LogLevel::Warn);
    assert!(LogLevel::Warn < LogLevel::Error);
}

// --- get_tool_id_color ---

#[test]
fn test_get_tool_id_color_deterministic() {
    assert_eq!(get_tool_id_color("abc"), get_tool_id_color("abc"));
}

#[test]
fn test_get_tool_id_color_valid_ansi() {
    assert!(get_tool_id_color("test").starts_with("\x1b[38;5;"));
}

// --- format_event_as_text ---

#[test]
fn test_format_event_init() {
    let event = Event::Init {
        model: "opus".to_string(),
        tools: vec![],
        working_directory: None,
        metadata: HashMap::new(),
    };
    let text = format_event_as_text(&event).unwrap();
    assert!(text.contains("opus"));
}

#[test]
fn test_format_event_assistant_text() {
    let event = Event::AssistantMessage {
        content: vec![ContentBlock::Text {
            text: "hello".to_string(),
        }],
        usage: None,
        parent_tool_use_id: None,
    };
    let text = format_event_as_text(&event).unwrap();
    assert!(text.contains("hello"));
}

#[test]
fn test_format_event_assistant_multiline() {
    let event = Event::AssistantMessage {
        content: vec![ContentBlock::Text {
            text: "line1\nline2\nline3".to_string(),
        }],
        usage: None,
        parent_tool_use_id: None,
    };
    let text = format_event_as_text(&event).unwrap();
    assert!(text.contains("line1"));
    assert!(text.contains("line2"));
    assert!(text.contains("line3"));
}

#[test]
fn test_format_event_assistant_empty() {
    let event = Event::AssistantMessage {
        content: vec![],
        usage: None,
        parent_tool_use_id: None,
    };
    assert!(format_event_as_text(&event).is_none());
}

#[test]
fn test_format_event_bash_tool_use() {
    let event = Event::AssistantMessage {
        content: vec![ContentBlock::ToolUse {
            id: "tool_abcd".to_string(),
            name: "Bash".to_string(),
            input: serde_json::json!({"command": "ls", "description": "List files"}),
        }],
        usage: None,
        parent_tool_use_id: None,
    };
    let text = format_event_as_text(&event).unwrap();
    assert!(text.contains("List files"));
    assert!(text.contains("ls"));
}

#[test]
fn test_format_event_bash_tool_use_no_description() {
    let event = Event::AssistantMessage {
        content: vec![ContentBlock::ToolUse {
            id: "tool_abcd".to_string(),
            name: "Bash".to_string(),
            input: serde_json::json!({"command": "echo hello"}),
        }],
        usage: None,
        parent_tool_use_id: None,
    };
    let text = format_event_as_text(&event).unwrap();
    assert!(text.contains("Run command")); // default description
    assert!(text.contains("echo hello"));
}

#[test]
fn test_format_event_non_bash_tool_use() {
    let event = Event::AssistantMessage {
        content: vec![ContentBlock::ToolUse {
            id: "tool_abcd".to_string(),
            name: "Read".to_string(),
            input: serde_json::json!({"file_path": "/tmp/test"}),
        }],
        usage: None,
        parent_tool_use_id: None,
    };
    let text = format_event_as_text(&event).unwrap();
    assert!(text.contains("Read"));
}

#[test]
fn test_format_event_tool_use_empty_input() {
    let event = Event::AssistantMessage {
        content: vec![ContentBlock::ToolUse {
            id: "tool_abcd".to_string(),
            name: "Read".to_string(),
            input: serde_json::json!({}),
        }],
        usage: None,
        parent_tool_use_id: None,
    };
    let text = format_event_as_text(&event).unwrap();
    assert!(text.contains("Read"));
}

#[test]
fn test_format_event_tool_use_various_types() {
    let event = Event::AssistantMessage {
        content: vec![ContentBlock::ToolUse {
            id: "tool_abcd".to_string(),
            name: "Custom".to_string(),
            input: serde_json::json!({
                "str_val": "hello",
                "num_val": 42,
                "bool_val": true,
                "null_val": null,
                "obj_val": {"nested": true}
            }),
        }],
        usage: None,
        parent_tool_use_id: None,
    };
    let text = format_event_as_text(&event).unwrap();
    assert!(text.contains("Custom"));
}

#[test]
fn test_format_event_tool_execution_success() {
    let event = Event::ToolExecution {
        tool_name: "Bash".to_string(),
        tool_id: "tool_abcd".to_string(),
        input: serde_json::json!({}),
        result: ToolResult {
            success: true,
            output: Some("file1\nfile2".to_string()),
            error: None,
            data: None,
        },
        parent_tool_use_id: None,
    };
    let text = format_event_as_text(&event).unwrap();
    assert!(text.contains("file1"));
    assert!(text.contains("file2"));
}

#[test]
fn test_format_event_tool_execution_failure() {
    let event = Event::ToolExecution {
        tool_name: "Bash".to_string(),
        tool_id: "tool_abcd".to_string(),
        input: serde_json::json!({}),
        result: ToolResult {
            success: false,
            output: None,
            error: Some("not found".to_string()),
            data: None,
        },
        parent_tool_use_id: None,
    };
    let text = format_event_as_text(&event).unwrap();
    assert!(text.contains("not found"));
}

#[test]
fn test_format_event_tool_execution_empty_output() {
    let event = Event::ToolExecution {
        tool_name: "Bash".to_string(),
        tool_id: "tool_abcd".to_string(),
        input: serde_json::json!({}),
        result: ToolResult {
            success: true,
            output: None,
            error: None,
            data: None,
        },
        parent_tool_use_id: None,
    };
    let text = format_event_as_text(&event).unwrap();
    assert!(text.contains("success"));
}

#[test]
fn test_format_event_result_returns_none() {
    let event = Event::Result {
        success: true,
        message: Some("done".to_string()),
        duration_ms: Some(100),
        num_turns: Some(1),
    };
    assert!(format_event_as_text(&event).is_none());
}

#[test]
fn test_format_event_error() {
    let event = Event::Error {
        message: "failed".to_string(),
        details: None,
    };
    let text = format_event_as_text(&event).unwrap();
    assert!(text.contains("failed"));
}

#[test]
fn test_format_event_permission() {
    let granted = Event::PermissionRequest {
        tool_name: "Bash".to_string(),
        description: "x".to_string(),
        granted: true,
    };
    assert!(format_event_as_text(&granted).unwrap().contains("granted"));

    let denied = Event::PermissionRequest {
        tool_name: "Bash".to_string(),
        description: "x".to_string(),
        granted: false,
    };
    assert!(format_event_as_text(&denied).unwrap().contains("denied"));
}

// --- Serialization ---

#[test]
fn test_agent_output_roundtrip() {
    let output = AgentOutput {
        agent: "claude".to_string(),
        session_id: "sess-1".to_string(),
        events: vec![Event::Init {
            model: "opus".to_string(),
            tools: vec!["Bash".to_string()],
            working_directory: Some("/tmp".to_string()),
            metadata: HashMap::new(),
        }],
        result: Some("done".to_string()),
        is_error: false,
        exit_code: None,
        error_message: None,
        total_cost_usd: Some(0.01),
        usage: Some(Usage {
            input_tokens: 100,
            output_tokens: 50,
            cache_read_tokens: None,
            cache_creation_tokens: None,
            web_search_requests: None,
            web_fetch_requests: None,
        }),
    };
    let json = serde_json::to_string(&output).unwrap();
    let parsed: AgentOutput = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.agent, "claude");
    assert_eq!(parsed.result, Some("done".to_string()));
    assert_eq!(parsed.exit_code, None);
    assert_eq!(parsed.error_message, None);
}

#[test]
fn test_agent_output_with_exit_info_roundtrip() {
    let output = AgentOutput {
        agent: "codex".to_string(),
        session_id: "sess-2".to_string(),
        events: vec![],
        result: None,
        is_error: true,
        exit_code: Some(2),
        error_message: Some("provider crashed".to_string()),
        total_cost_usd: None,
        usage: None,
    };
    let json = serde_json::to_string(&output).unwrap();
    let parsed: AgentOutput = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.exit_code, Some(2));
    assert_eq!(parsed.error_message, Some("provider crashed".to_string()));
    assert!(parsed.is_error);
}

#[test]
fn test_agent_output_skip_serializing_none_exit_fields() {
    let output = AgentOutput {
        agent: "test".to_string(),
        session_id: String::new(),
        events: vec![],
        result: None,
        is_error: false,
        exit_code: None,
        error_message: None,
        total_cost_usd: None,
        usage: None,
    };
    let json = serde_json::to_string(&output).unwrap();
    assert!(!json.contains("exit_code"));
    assert!(!json.contains("error_message"));
}

#[test]
fn test_agent_output_deserialize_without_exit_fields() {
    // Backwards compatibility: JSON without exit_code/error_message should still parse
    let json = r#"{"agent":"test","session_id":"","events":[],"result":null,"is_error":false,"total_cost_usd":null,"usage":null}"#;
    let parsed: AgentOutput = serde_json::from_str(json).unwrap();
    assert_eq!(parsed.exit_code, None);
    assert_eq!(parsed.error_message, None);
}

#[test]
fn test_format_event_long_string_truncation() {
    let long_string = "a".repeat(100);
    let event = Event::AssistantMessage {
        content: vec![ContentBlock::ToolUse {
            id: "tool_abcd".to_string(),
            name: "Write".to_string(),
            input: serde_json::json!({"content": long_string}),
        }],
        usage: None,
        parent_tool_use_id: None,
    };
    let text = format_event_as_text(&event).unwrap();
    assert!(text.contains("..."));
}

// --- UserMessage event tests ---

#[test]
fn test_event_to_log_entry_user_message() {
    let event = Event::UserMessage {
        content: vec![ContentBlock::Text {
            text: "hello world".to_string(),
        }],
    };
    let entry = event_to_log_entry(&event).unwrap();
    assert_eq!(entry.level, LogLevel::Info);
    assert!(entry.message.contains("hello world"));
}

#[test]
fn test_event_to_log_entry_user_message_empty() {
    let event = Event::UserMessage { content: vec![] };
    assert!(event_to_log_entry(&event).is_none());
}

#[test]
fn test_event_to_log_entry_user_message_tool_use_only() {
    let event = Event::UserMessage {
        content: vec![ContentBlock::ToolUse {
            id: "id1".to_string(),
            name: "Bash".to_string(),
            input: serde_json::json!({}),
        }],
    };
    assert!(event_to_log_entry(&event).is_none());
}

#[test]
fn test_event_to_log_entry_user_message_multiple_blocks() {
    let event = Event::UserMessage {
        content: vec![
            ContentBlock::Text {
                text: "line one".to_string(),
            },
            ContentBlock::Text {
                text: "line two".to_string(),
            },
        ],
    };
    let entry = event_to_log_entry(&event).unwrap();
    assert!(entry.message.contains("line one"));
    assert!(entry.message.contains("line two"));
}

#[test]
fn test_format_event_user_message() {
    let event = Event::UserMessage {
        content: vec![ContentBlock::Text {
            text: "hello world".to_string(),
        }],
    };
    let text = format_event_as_text(&event).unwrap();
    assert!(text.contains("hello world"));
    assert!(text.contains("> "));
}

#[test]
fn test_format_event_user_message_empty() {
    let event = Event::UserMessage { content: vec![] };
    assert!(format_event_as_text(&event).is_none());
}

// --- Tool execution edge cases ---

#[test]
fn test_event_to_log_entry_tool_execution_no_error_message() {
    let event = Event::ToolExecution {
        tool_name: "Custom".to_string(),
        tool_id: "id1".to_string(),
        input: serde_json::json!({}),
        result: ToolResult {
            success: false,
            output: None,
            error: None,
            data: None,
        },
        parent_tool_use_id: None,
    };
    let entry = event_to_log_entry(&event).unwrap();
    assert_eq!(entry.level, LogLevel::Warn);
    assert!(entry.message.contains("unknown error"));
}
