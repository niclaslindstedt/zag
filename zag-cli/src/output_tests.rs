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
        model: None,
        provider: None,
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
    };
    assert!(event_to_log_entry(&event).is_none());
}

#[test]
fn test_event_to_log_entry_empty_content_returns_none() {
    let event = Event::AssistantMessage {
        content: vec![],
        usage: None,
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
        model: Some("opus".to_string()),
        provider: Some("claude".to_string()),
    };
    let json = serde_json::to_string(&output).unwrap();
    let parsed: AgentOutput = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.agent, "claude");
    assert_eq!(parsed.result, Some("done".to_string()));
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
    };
    let text = format_event_as_text(&event).unwrap();
    assert!(text.contains("..."));
}
