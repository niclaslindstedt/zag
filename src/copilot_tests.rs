use super::{Copilot, parse_copilot_event_line};
use crate::sandbox::SandboxConfig;
use crate::session_log::LogEventKind;
use std::collections::HashSet;

#[test]
fn test_build_run_args_non_interactive() {
    let mut copilot = Copilot::new();
    copilot.model = "claude-sonnet-4.5".to_string();

    let args = copilot.build_run_args(false, Some("hello"));
    assert!(args.contains(&"--allow-all-tools".to_string()));
    assert!(args.contains(&"--model".to_string()));
    assert!(args.contains(&"claude-sonnet-4.5".to_string()));
    assert!(args.contains(&"-p".to_string()));
    assert!(args.contains(&"hello".to_string()));
}

#[test]
fn test_build_run_args_interactive_with_prompt() {
    let copilot = Copilot::new();
    let args = copilot.build_run_args(true, Some("hello"));
    assert!(!args.contains(&"--allow-all-tools".to_string()));
    assert!(args.contains(&"-i".to_string()));
    assert!(args.contains(&"hello".to_string()));
}

#[test]
fn test_build_run_args_interactive_no_prompt() {
    let copilot = Copilot::new();
    let args = copilot.build_run_args(true, None);
    assert!(!args.contains(&"-i".to_string()));
    assert!(!args.contains(&"-p".to_string()));
}

#[test]
fn test_build_run_args_skip_permissions() {
    let mut copilot = Copilot::new();
    copilot.skip_permissions = true;

    let args = copilot.build_run_args(true, None);
    assert!(args.contains(&"--allow-all-tools".to_string()));
}

#[test]
fn test_build_run_args_add_dirs() {
    let mut copilot = Copilot::new();
    copilot.add_dirs = vec!["/extra".to_string()];

    let args = copilot.build_run_args(true, None);
    assert!(args.contains(&"--add-dir".to_string()));
    assert!(args.contains(&"/extra".to_string()));
}

#[test]
fn test_make_command_without_sandbox() {
    let mut copilot = Copilot::new();
    copilot.root = Some("/project".to_string());

    let cmd = copilot.make_command(vec!["-p".to_string(), "hello".to_string()]);
    assert_eq!(cmd.as_std().get_program().to_str().unwrap(), "copilot");
    assert_eq!(
        cmd.as_std().get_current_dir().unwrap().to_str().unwrap(),
        "/project"
    );
}

#[test]
fn test_make_command_with_sandbox() {
    let mut copilot = Copilot::new();
    copilot.sandbox = Some(SandboxConfig {
        name: "sandbox-cp".to_string(),
        template: "docker/sandbox-templates:copilot".to_string(),
        workspace: "/workspace".to_string(),
    });

    let cmd = copilot.make_command(vec!["-p".to_string(), "hello".to_string()]);
    assert_eq!(cmd.as_std().get_program().to_str().unwrap(), "docker");
    let args: Vec<&str> = cmd
        .as_std()
        .get_args()
        .map(|a| a.to_str().unwrap())
        .collect();
    assert!(args.contains(&"sandbox"));
    assert!(args.contains(&"run"));
    assert!(args.contains(&"sandbox-cp"));
    assert!(args.contains(&"-p"));
    assert!(args.contains(&"hello"));
}

#[test]
fn test_parse_copilot_assistant_message_event() {
    let line = r#"{"type":"assistant.message","data":{"messageId":"msg-1","content":"hello","toolRequests":[{"toolCallId":"tool-1","name":"view","arguments":{"path":"CLAUDE.md"}}]},"id":"evt-1","timestamp":"2026-01-14T12:41:41.008Z","parentId":null}"#;
    let mut seen = HashSet::new();

    let parsed = parse_copilot_event_line(line, &mut seen).expect("parsed event");

    assert_eq!(parsed.events.len(), 2);
    match &parsed.events[0] {
        LogEventKind::AssistantMessage {
            content,
            message_id,
        } => {
            assert_eq!(content, "hello");
            assert_eq!(message_id.as_deref(), Some("msg-1"));
        }
        other => panic!("unexpected event: {other:?}"),
    }
    match &parsed.events[1] {
        LogEventKind::ToolCall {
            tool_name,
            tool_id,
            input,
        } => {
            assert_eq!(tool_name, "view");
            assert_eq!(tool_id.as_deref(), Some("tool-1"));
            assert_eq!(
                input
                    .as_ref()
                    .and_then(|value| value.get("path"))
                    .and_then(|value| value.as_str()),
                Some("CLAUDE.md")
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[test]
fn test_parse_copilot_tool_result_event() {
    let line = r#"{"type":"tool.execution_complete","data":{"toolCallId":"tool-2","toolName":"bash","success":true,"result":{"content":"ok"}},"id":"evt-2","timestamp":"2026-01-14T12:41:41.008Z","parentId":null}"#;
    let mut seen = HashSet::new();

    let parsed = parse_copilot_event_line(line, &mut seen).expect("parsed event");

    assert_eq!(parsed.events.len(), 1);
    match &parsed.events[0] {
        LogEventKind::ToolResult {
            tool_name,
            tool_id,
            success,
            output,
            error,
            ..
        } => {
            assert_eq!(tool_name.as_deref(), Some("bash"));
            assert_eq!(tool_id.as_deref(), Some("tool-2"));
            assert_eq!(*success, Some(true));
            assert_eq!(output.as_deref(), Some("ok"));
            assert_eq!(error, &None);
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[test]
fn test_parse_copilot_session_start_metadata() {
    let line = r#"{"type":"session.start","data":{"sessionId":"session-1","selectedModel":"claude-sonnet-4.5","context":{"cwd":"/repo"}},"id":"evt-3","timestamp":"2026-01-14T12:40:56.938Z","parentId":null}"#;
    let mut seen = HashSet::new();

    let parsed = parse_copilot_event_line(line, &mut seen).expect("parsed event");

    assert_eq!(parsed.provider_session_id.as_deref(), Some("session-1"));
    assert_eq!(parsed.model.as_deref(), Some("claude-sonnet-4.5"));
    assert_eq!(parsed.workspace_path.as_deref(), Some("/repo"));
    assert!(matches!(
        parsed.events.first(),
        Some(LogEventKind::ProviderStatus { .. })
    ));
}

#[test]
fn test_parse_copilot_event_dedupes_ids() {
    let line = r#"{"type":"user.message","data":{"content":"hello"},"id":"evt-4","timestamp":"2026-01-14T12:40:56.938Z","parentId":null}"#;
    let mut seen = HashSet::new();

    assert!(parse_copilot_event_line(line, &mut seen).is_some());
    assert!(parse_copilot_event_line(line, &mut seen).is_none());
}
