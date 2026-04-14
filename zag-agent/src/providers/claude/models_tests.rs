use super::*;

#[test]
fn test_parse_simple_response() {
    let json = r#"[
        {
            "type": "system",
            "subtype": "init",
            "session_id": "test123",
            "model": "claude-sonnet-4-5",
            "tools": ["Bash", "Read"],
            "uuid": "uuid1"
        },
        {
            "type": "assistant",
            "message": {
                "model": "claude-sonnet-4-5",
                "id": "msg1",
                "type": "message",
                "role": "assistant",
                "content": [
                    {"type": "text", "text": "Hello world"}
                ],
                "stop_reason": null,
                "stop_sequence": null,
                "usage": {
                    "input_tokens": 10,
                    "output_tokens": 5
                }
            },
            "parent_tool_use_id": null,
            "session_id": "test123",
            "uuid": "uuid2"
        },
        {
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 1000,
            "duration_api_ms": 950,
            "num_turns": 1,
            "result": "Hello world",
            "session_id": "test123",
            "total_cost_usd": 0.001,
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5
            },
            "permission_denials": [],
            "uuid": "uuid3"
        }
    ]"#;

    let claude_output: ClaudeOutput = serde_json::from_str(json).expect("Failed to parse");
    let agent_output: AgentOutput = claude_output_to_agent_output(claude_output);

    assert_eq!(agent_output.agent, "claude");
    assert_eq!(agent_output.session_id, "test123");
    assert_eq!(agent_output.result, Some("Hello world".to_string()));
    assert!(!agent_output.is_error);
    assert_eq!(agent_output.total_cost_usd, Some(0.001));
}

#[test]
fn test_parse_with_tool_use() {
    let json = r#"[
        {
            "type": "system",
            "subtype": "init",
            "session_id": "sess1",
            "model": "opus",
            "tools": ["Bash"],
            "uuid": "u1"
        },
        {
            "type": "assistant",
            "message": {
                "model": "opus",
                "id": "msg1",
                "type": "message",
                "role": "assistant",
                "content": [
                    {"type": "text", "text": "Let me check."},
                    {"type": "tool_use", "id": "tool1", "name": "Bash", "input": {"command": "ls"}}
                ],
                "stop_reason": "tool_use",
                "stop_sequence": null,
                "usage": {"input_tokens": 100, "output_tokens": 50}
            },
            "parent_tool_use_id": null,
            "session_id": "sess1",
            "uuid": "u2"
        },
        {
            "type": "user",
            "message": {
                "role": "user",
                "content": [
                    {"tool_use_id": "tool1", "type": "tool_result", "content": "file1.rs\nfile2.rs", "is_error": false}
                ]
            },
            "parent_tool_use_id": null,
            "session_id": "sess1",
            "uuid": "u3",
            "tool_use_result": null
        },
        {
            "type": "assistant",
            "message": {
                "model": "opus",
                "id": "msg2",
                "type": "message",
                "role": "assistant",
                "content": [
                    {"type": "text", "text": "Found 2 files."}
                ],
                "stop_reason": "end_turn",
                "stop_sequence": null,
                "usage": {"input_tokens": 200, "output_tokens": 30}
            },
            "parent_tool_use_id": null,
            "session_id": "sess1",
            "uuid": "u4"
        },
        {
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 2000,
            "duration_api_ms": 1800,
            "num_turns": 2,
            "result": "Found 2 files.",
            "session_id": "sess1",
            "total_cost_usd": 0.005,
            "usage": {"input_tokens": 300, "output_tokens": 80},
            "permission_denials": [],
            "uuid": "u5"
        }
    ]"#;

    let claude_output: ClaudeOutput = serde_json::from_str(json).expect("Failed to parse");
    let agent_output: AgentOutput = claude_output_to_agent_output(claude_output);

    assert_eq!(agent_output.session_id, "sess1");
    assert_eq!(agent_output.result, Some("Found 2 files.".to_string()));

    // Should have: Init, AssistantMessage (with tool_use), ToolExecution,
    // AssistantMessage, TurnComplete, Result
    assert_eq!(agent_output.events.len(), 6);

    // Verify tool execution was extracted
    let tool_execs = agent_output.tool_executions();
    assert_eq!(tool_execs.len(), 1);
    if let crate::output::Event::ToolExecution { tool_name, .. } = tool_execs[0] {
        assert_eq!(tool_name, "Bash");
    }
}

#[test]
fn test_parse_error_response() {
    let json = r#"[
        {
            "type": "system",
            "subtype": "init",
            "session_id": "sess1",
            "model": "opus",
            "tools": [],
            "uuid": "u1"
        },
        {
            "type": "result",
            "subtype": "error",
            "is_error": true,
            "duration_ms": 500,
            "duration_api_ms": 400,
            "num_turns": 0,
            "result": "Something went wrong",
            "session_id": "sess1",
            "total_cost_usd": 0.0,
            "usage": {"input_tokens": 5, "output_tokens": 0},
            "permission_denials": [],
            "uuid": "u2"
        }
    ]"#;

    let claude_output: ClaudeOutput = serde_json::from_str(json).expect("Failed to parse");
    let agent_output: AgentOutput = claude_output_to_agent_output(claude_output);

    assert!(agent_output.is_error);
    assert_eq!(
        agent_output.result,
        Some("Something went wrong".to_string())
    );
}

#[test]
fn test_parse_with_permission_denials() {
    let json = r#"[
        {
            "type": "system",
            "subtype": "init",
            "session_id": "sess1",
            "model": "opus",
            "tools": ["Bash"],
            "uuid": "u1"
        },
        {
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 1000,
            "duration_api_ms": 900,
            "num_turns": 1,
            "result": "Done",
            "session_id": "sess1",
            "total_cost_usd": 0.001,
            "usage": {"input_tokens": 10, "output_tokens": 5},
            "permission_denials": [
                {"tool_name": "Bash", "tool_use_id": "t1", "tool_input": {"command": "rm -rf /"}}
            ],
            "uuid": "u2"
        }
    ]"#;

    let claude_output: ClaudeOutput = serde_json::from_str(json).expect("Failed to parse");
    let agent_output: AgentOutput = claude_output_to_agent_output(claude_output);

    // Should have Init, PermissionRequest, Result
    let perm_events: Vec<_> = agent_output
        .events
        .iter()
        .filter(|e| matches!(e, crate::output::Event::PermissionRequest { .. }))
        .collect();
    assert_eq!(perm_events.len(), 1);
    if let crate::output::Event::PermissionRequest {
        tool_name, granted, ..
    } = perm_events[0]
    {
        assert_eq!(tool_name, "Bash");
        assert!(!granted);
    }
}

#[test]
fn test_parse_with_tool_error() {
    let json = r#"[
        {
            "type": "system",
            "subtype": "init",
            "session_id": "sess1",
            "model": "opus",
            "tools": ["Bash"],
            "uuid": "u1"
        },
        {
            "type": "assistant",
            "message": {
                "model": "opus",
                "id": "msg1",
                "type": "message",
                "role": "assistant",
                "content": [
                    {"type": "tool_use", "id": "tool1", "name": "Bash", "input": {"command": "bad-cmd"}}
                ],
                "stop_reason": "tool_use",
                "stop_sequence": null,
                "usage": {"input_tokens": 50, "output_tokens": 20}
            },
            "parent_tool_use_id": null,
            "session_id": "sess1",
            "uuid": "u2"
        },
        {
            "type": "user",
            "message": {
                "role": "user",
                "content": [
                    {"tool_use_id": "tool1", "type": "tool_result", "content": "command not found", "is_error": true}
                ]
            },
            "parent_tool_use_id": null,
            "session_id": "sess1",
            "uuid": "u3",
            "tool_use_result": null
        },
        {
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 1000,
            "duration_api_ms": 900,
            "num_turns": 1,
            "result": "Failed to run command",
            "session_id": "sess1",
            "total_cost_usd": 0.001,
            "usage": {"input_tokens": 100, "output_tokens": 40},
            "permission_denials": [],
            "uuid": "u4"
        }
    ]"#;

    let claude_output: ClaudeOutput = serde_json::from_str(json).expect("Failed to parse");
    let agent_output: AgentOutput = claude_output_to_agent_output(claude_output);

    let tool_execs = agent_output.tool_executions();
    assert_eq!(tool_execs.len(), 1);
    if let crate::output::Event::ToolExecution { result, .. } = tool_execs[0] {
        assert!(!result.success);
        assert_eq!(result.error.as_deref(), Some("command not found"));
    }
}

#[test]
fn test_parse_with_usage_details() {
    let json = r#"[
        {
            "type": "system",
            "subtype": "init",
            "session_id": "sess1",
            "model": "opus",
            "tools": [],
            "uuid": "u1"
        },
        {
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 1000,
            "duration_api_ms": 900,
            "num_turns": 1,
            "result": "ok",
            "session_id": "sess1",
            "total_cost_usd": 0.01,
            "usage": {
                "input_tokens": 500,
                "output_tokens": 200,
                "cache_read_input_tokens": 100,
                "cache_creation_input_tokens": 50,
                "server_tool_use": {
                    "web_search_requests": 2,
                    "web_fetch_requests": 1
                }
            },
            "permission_denials": [],
            "uuid": "u2"
        }
    ]"#;

    let claude_output: ClaudeOutput = serde_json::from_str(json).expect("Failed to parse");
    let agent_output: AgentOutput = claude_output_to_agent_output(claude_output);

    let usage = agent_output.usage.unwrap();
    assert_eq!(usage.input_tokens, 500);
    assert_eq!(usage.output_tokens, 200);
    assert_eq!(usage.cache_read_tokens, Some(100));
    assert_eq!(usage.cache_creation_tokens, Some(50));
    assert_eq!(usage.web_search_requests, Some(2));
    assert_eq!(usage.web_fetch_requests, Some(1));
}

#[test]
fn test_find_tool_name() {
    use crate::output::{ContentBlock as UB, Event as UE};

    let events = vec![UE::AssistantMessage {
        content: vec![
            UB::ToolUse {
                id: "t1".to_string(),
                name: "Bash".to_string(),
                input: serde_json::json!({}),
            },
            UB::ToolUse {
                id: "t2".to_string(),
                name: "Read".to_string(),
                input: serde_json::json!({}),
            },
        ],
        usage: None,
        parent_tool_use_id: None,
    }];

    assert_eq!(find_tool_name(&events, "t1"), Some("Bash".to_string()));
    assert_eq!(find_tool_name(&events, "t2"), Some("Read".to_string()));
    assert_eq!(find_tool_name(&events, "t3"), None);
}

#[test]
fn test_parse_user_message_with_mixed_content_blocks() {
    let json = r#"[
        {
            "type": "system",
            "subtype": "init",
            "session_id": "sess1",
            "model": "opus",
            "tools": ["Bash"],
            "uuid": "u1"
        },
        {
            "type": "assistant",
            "message": {
                "model": "opus",
                "id": "msg1",
                "type": "message",
                "role": "assistant",
                "content": [
                    {"type": "tool_use", "id": "tool1", "name": "Bash", "input": {"command": "echo hi"}}
                ],
                "stop_reason": "tool_use",
                "stop_sequence": null,
                "usage": {"input_tokens": 50, "output_tokens": 20}
            },
            "parent_tool_use_id": null,
            "session_id": "sess1",
            "uuid": "u2"
        },
        {
            "type": "user",
            "message": {
                "role": "user",
                "content": [
                    {"type": "text", "text": "some injected text"},
                    {"type": "tool_result", "tool_use_id": "tool1", "content": "hi", "is_error": false}
                ]
            },
            "parent_tool_use_id": null,
            "session_id": "sess1",
            "uuid": "u3",
            "tool_use_result": null
        },
        {
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 500,
            "duration_api_ms": 400,
            "num_turns": 1,
            "result": "Done",
            "session_id": "sess1",
            "total_cost_usd": 0.001,
            "usage": {"input_tokens": 100, "output_tokens": 30},
            "permission_denials": [],
            "uuid": "u4"
        }
    ]"#;

    let claude_output: ClaudeOutput = serde_json::from_str(json).expect("Failed to parse");
    let agent_output: AgentOutput = claude_output_to_agent_output(claude_output);

    // Text blocks in user messages should be skipped; only tool_result produces ToolExecution
    let tool_execs = agent_output.tool_executions();
    assert_eq!(tool_execs.len(), 1);
    if let crate::output::Event::ToolExecution {
        tool_name, result, ..
    } = tool_execs[0]
    {
        assert_eq!(tool_name, "Bash");
        assert!(result.success);
        assert_eq!(result.output.as_deref(), Some("hi"));
    }
}

#[test]
fn test_parse_system_event_with_cwd() {
    let json = r#"[
        {
            "type": "system",
            "subtype": "init",
            "session_id": "sess1",
            "cwd": "/home/user/project",
            "model": "opus",
            "tools": ["Bash", "Read", "Write"],
            "uuid": "u1",
            "permissionMode": "default"
        },
        {
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 100,
            "duration_api_ms": 90,
            "num_turns": 0,
            "result": "",
            "session_id": "sess1",
            "total_cost_usd": 0.0,
            "usage": {"input_tokens": 0, "output_tokens": 0},
            "permission_denials": [],
            "uuid": "u2"
        }
    ]"#;

    let claude_output: ClaudeOutput = serde_json::from_str(json).expect("Failed to parse");
    let agent_output: AgentOutput = claude_output_to_agent_output(claude_output);

    if let crate::output::Event::Init {
        working_directory,
        tools,
        model,
        ..
    } = &agent_output.events[0]
    {
        assert_eq!(working_directory.as_deref(), Some("/home/user/project"));
        assert_eq!(tools.len(), 3);
        assert_eq!(model, "opus");
    } else {
        panic!("Expected Init event");
    }
}

#[test]
fn test_empty_result_falls_back_to_assistant_text() {
    let json = r#"[
        {
            "type": "system",
            "subtype": "init",
            "session_id": "sess1",
            "model": "sonnet",
            "tools": [],
            "uuid": "u1"
        },
        {
            "type": "assistant",
            "message": {
                "model": "sonnet",
                "id": "msg1",
                "type": "message",
                "role": "assistant",
                "content": [
                    {"type": "text", "text": "{\"name\":\"my-project\",\"language\":\"rust\"}"}
                ],
                "stop_reason": "end_turn",
                "stop_sequence": null,
                "usage": {"input_tokens": 100, "output_tokens": 40}
            },
            "parent_tool_use_id": null,
            "session_id": "sess1",
            "uuid": "u2"
        },
        {
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 1000,
            "duration_api_ms": 900,
            "num_turns": 1,
            "result": "",
            "session_id": "sess1",
            "total_cost_usd": 0.001,
            "usage": {"input_tokens": 100, "output_tokens": 40},
            "permission_denials": [],
            "uuid": "u3"
        }
    ]"#;

    let claude_output: ClaudeOutput = serde_json::from_str(json).expect("Failed to parse");
    let agent_output: AgentOutput = claude_output_to_agent_output(claude_output);

    // Result.result was empty, so the fallback should use the assistant text.
    assert_eq!(
        agent_output.result,
        Some("{\"name\":\"my-project\",\"language\":\"rust\"}".to_string())
    );
    assert!(!agent_output.is_error);
}

#[test]
fn test_nonempty_result_not_overridden_by_assistant() {
    let json = r#"[
        {
            "type": "system",
            "subtype": "init",
            "session_id": "sess1",
            "model": "sonnet",
            "tools": [],
            "uuid": "u1"
        },
        {
            "type": "assistant",
            "message": {
                "model": "sonnet",
                "id": "msg1",
                "type": "message",
                "role": "assistant",
                "content": [
                    {"type": "text", "text": "some intermediate text"}
                ],
                "stop_reason": "end_turn",
                "stop_sequence": null,
                "usage": {"input_tokens": 50, "output_tokens": 20}
            },
            "parent_tool_use_id": null,
            "session_id": "sess1",
            "uuid": "u2"
        },
        {
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 500,
            "duration_api_ms": 400,
            "num_turns": 1,
            "result": "the real result",
            "session_id": "sess1",
            "total_cost_usd": 0.001,
            "usage": {"input_tokens": 50, "output_tokens": 20},
            "permission_denials": [],
            "uuid": "u3"
        }
    ]"#;

    let claude_output: ClaudeOutput = serde_json::from_str(json).expect("Failed to parse");
    let agent_output: AgentOutput = claude_output_to_agent_output(claude_output);

    // Non-empty result should NOT be overridden by assistant text.
    assert_eq!(agent_output.result, Some("the real result".to_string()));
}

#[test]
fn test_structured_output_fallback_when_result_empty() {
    // Reproduces the bug where Claude CLI with --json-schema puts the data
    // in structured_output while leaving result empty, and the assistant
    // message only has ToolUse blocks (StructuredOutput tool), no Text.
    let json = r#"[
        {
            "type": "system",
            "subtype": "init",
            "session_id": "sess1",
            "model": "sonnet",
            "tools": [],
            "uuid": "u1"
        },
        {
            "type": "assistant",
            "message": {
                "model": "sonnet",
                "id": "msg1",
                "type": "message",
                "role": "assistant",
                "content": [
                    {"type": "tool_use", "id": "tool1", "name": "StructuredOutput", "input": {"name": "my-project"}}
                ],
                "stop_reason": null,
                "stop_sequence": null,
                "usage": {"input_tokens": 100, "output_tokens": 40}
            },
            "parent_tool_use_id": null,
            "session_id": "sess1",
            "uuid": "u2"
        },
        {
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 1000,
            "duration_api_ms": 900,
            "num_turns": 1,
            "result": "",
            "session_id": "sess1",
            "total_cost_usd": 0.001,
            "usage": {"input_tokens": 100, "output_tokens": 40},
            "permission_denials": [],
            "structured_output": {"name": "my-project"},
            "uuid": "u3"
        }
    ]"#;

    let claude_output: ClaudeOutput = serde_json::from_str(json).expect("Failed to parse");
    let agent_output: AgentOutput = claude_output_to_agent_output(claude_output);

    // Result.result was empty and no assistant text, so structured_output should be used.
    assert_eq!(
        agent_output.result,
        Some(r#"{"name":"my-project"}"#.to_string())
    );
    assert!(!agent_output.is_error);
}

#[test]
fn test_unknown_content_block_type_ignored() {
    // Ensures that unknown content block types don't break deserialization.
    let json = r#"[
        {
            "type": "system",
            "subtype": "init",
            "session_id": "sess1",
            "model": "sonnet",
            "tools": [],
            "uuid": "u1"
        },
        {
            "type": "assistant",
            "message": {
                "model": "sonnet",
                "id": "msg1",
                "type": "message",
                "role": "assistant",
                "content": [
                    {"type": "text", "text": "hello"},
                    {"type": "some_future_block_type", "data": 42}
                ],
                "stop_reason": "end_turn",
                "stop_sequence": null,
                "usage": {"input_tokens": 50, "output_tokens": 10}
            },
            "parent_tool_use_id": null,
            "session_id": "sess1",
            "uuid": "u2"
        },
        {
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 500,
            "duration_api_ms": 400,
            "num_turns": 1,
            "result": "hello",
            "session_id": "sess1",
            "total_cost_usd": 0.001,
            "usage": {"input_tokens": 50, "output_tokens": 10},
            "permission_denials": [],
            "uuid": "u3"
        }
    ]"#;

    let claude_output: ClaudeOutput = serde_json::from_str(json).expect("Failed to parse");
    let agent_output: AgentOutput = claude_output_to_agent_output(claude_output);

    // Unknown block type should be silently ignored, text block still works.
    assert_eq!(agent_output.result, Some("hello".to_string()));
}
