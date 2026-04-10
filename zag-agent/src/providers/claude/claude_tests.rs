use super::Claude;
use crate::sandbox::SandboxConfig;

#[test]
fn test_build_run_args_non_interactive() {
    let mut claude = Claude::new();
    claude.common.model = "opus".to_string();

    let fmt = Some("json".to_string());
    let args = claude.build_run_args(false, Some("hello"), &fmt);

    assert!(args.contains(&"--print".to_string()));
    assert!(args.contains(&"--verbose".to_string()));
    assert!(args.contains(&"--model".to_string()));
    assert!(args.contains(&"opus".to_string()));
    assert!(args.contains(&"hello".to_string()));
}

#[test]
fn test_build_run_args_interactive() {
    let claude = Claude::new();
    let args = claude.build_run_args(true, Some("hello"), &None);

    assert!(!args.contains(&"--print".to_string()));
    assert!(!args.contains(&"--verbose".to_string()));
    assert!(args.contains(&"hello".to_string()));
}

#[test]
fn test_build_run_args_skip_permissions() {
    let mut claude = Claude::new();
    claude.common.skip_permissions = true;

    let args = claude.build_run_args(true, None, &None);
    assert!(args.contains(&"--dangerously-skip-permissions".to_string()));
}

#[test]
fn test_build_run_args_sandbox_skips_permissions() {
    let mut claude = Claude::new();
    claude.common.skip_permissions = true;
    claude.common.sandbox = Some(SandboxConfig {
        name: "test".to_string(),
        template: "docker/sandbox-templates:claude-code".to_string(),
        workspace: "/workspace".to_string(),
    });

    let args = claude.build_run_args(true, None, &None);
    assert!(!args.contains(&"--dangerously-skip-permissions".to_string()));
}

#[test]
fn test_build_run_args_with_system_prompt() {
    let mut claude = Claude::new();
    claude.common.system_prompt = "You are helpful".to_string();

    let args = claude.build_run_args(true, None, &None);
    assert!(args.contains(&"--append-system-prompt".to_string()));
    assert!(args.contains(&"You are helpful".to_string()));
}

#[test]
fn test_build_run_args_input_format_only_non_interactive() {
    let mut claude = Claude::new();
    claude.input_format = Some("stream-json".to_string());

    let interactive_args = claude.build_run_args(true, None, &None);
    assert!(!interactive_args.contains(&"--input-format".to_string()));

    let non_interactive_args = claude.build_run_args(false, Some("hello"), &None);
    assert!(non_interactive_args.contains(&"--input-format".to_string()));
    assert!(non_interactive_args.contains(&"stream-json".to_string()));
}

#[test]
fn test_build_run_args_replay_user_messages_only_non_interactive() {
    let mut claude = Claude::new();
    claude.replay_user_messages = true;

    let interactive_args = claude.build_run_args(true, None, &None);
    assert!(!interactive_args.contains(&"--replay-user-messages".to_string()));

    let non_interactive_args = claude.build_run_args(false, Some("hello"), &None);
    assert!(non_interactive_args.contains(&"--replay-user-messages".to_string()));
}

#[test]
fn test_build_run_args_include_partial_messages_only_non_interactive() {
    let mut claude = Claude::new();
    claude.include_partial_messages = true;

    let interactive_args = claude.build_run_args(true, None, &None);
    assert!(!interactive_args.contains(&"--include-partial-messages".to_string()));

    let non_interactive_args = claude.build_run_args(false, Some("hello"), &None);
    assert!(non_interactive_args.contains(&"--include-partial-messages".to_string()));
}

#[test]
fn test_build_resume_args() {
    let mut claude = Claude::new();
    claude.common.model = "sonnet".to_string();

    let args = claude.build_resume_args(Some("session-123"));
    assert!(args.contains(&"--resume".to_string()));
    assert!(args.contains(&"session-123".to_string()));
    assert!(args.contains(&"--model".to_string()));
    assert!(args.contains(&"sonnet".to_string()));
}

#[test]
fn test_build_resume_args_continue() {
    let claude = Claude::new();
    let args = claude.build_resume_args(None);
    assert!(args.contains(&"--continue".to_string()));
    assert!(!args.contains(&"--resume".to_string()));
}

#[test]
fn test_build_resume_args_sandbox_skips_permissions() {
    let mut claude = Claude::new();
    claude.common.skip_permissions = true;
    claude.common.sandbox = Some(SandboxConfig {
        name: "test".to_string(),
        template: "docker/sandbox-templates:claude-code".to_string(),
        workspace: "/workspace".to_string(),
    });

    let args = claude.build_resume_args(Some("sid"));
    assert!(!args.contains(&"--dangerously-skip-permissions".to_string()));
}

#[test]
fn test_build_streaming_resume_args() {
    let mut claude = Claude::new();
    claude.common.model = "sonnet".to_string();

    let args = claude.build_streaming_resume_args("session-456");
    assert!(args.contains(&"--print".to_string()));
    assert!(args.contains(&"--resume".to_string()));
    assert!(args.contains(&"session-456".to_string()));
    assert!(args.contains(&"--output-format".to_string()));
    assert!(args.contains(&"stream-json".to_string()));
    assert!(args.contains(&"--input-format".to_string()));
    assert!(args.contains(&"--replay-user-messages".to_string()));
    assert!(args.contains(&"--model".to_string()));
    assert!(args.contains(&"sonnet".to_string()));
    assert!(!args.contains(&"--include-partial-messages".to_string()));
}

#[test]
fn test_build_streaming_resume_args_with_partial_messages() {
    let mut claude = Claude::new();
    claude.common.model = "opus".to_string();
    claude.include_partial_messages = true;

    let args = claude.build_streaming_resume_args("session-789");
    assert!(args.contains(&"--include-partial-messages".to_string()));
    assert!(args.contains(&"--replay-user-messages".to_string()));
}

#[test]
fn test_build_streaming_resume_args_sandbox_skips_permissions() {
    let mut claude = Claude::new();
    claude.common.skip_permissions = true;
    claude.common.sandbox = Some(SandboxConfig {
        name: "test".to_string(),
        template: "docker/sandbox-templates:claude-code".to_string(),
        workspace: "/workspace".to_string(),
    });

    let args = claude.build_streaming_resume_args("sid");
    assert!(!args.contains(&"--dangerously-skip-permissions".to_string()));
    assert!(args.contains(&"--resume".to_string()));
    assert!(args.contains(&"--replay-user-messages".to_string()));
}

#[test]
fn test_make_command_without_sandbox() {
    let mut claude = Claude::new();
    claude.common.root = Some("/project".to_string());

    let cmd = claude.make_command(vec!["--print".to_string(), "hello".to_string()]);
    assert_eq!(cmd.as_std().get_program().to_str().unwrap(), "claude");
    assert_eq!(
        cmd.as_std().get_current_dir().unwrap().to_str().unwrap(),
        "/project"
    );
}

#[test]
fn test_make_command_with_sandbox() {
    let mut claude = Claude::new();
    claude.common.sandbox = Some(SandboxConfig {
        name: "sandbox-abc".to_string(),
        template: "docker/sandbox-templates:claude-code".to_string(),
        workspace: "/workspace".to_string(),
    });

    let cmd = claude.make_command(vec!["--print".to_string(), "hello".to_string()]);
    assert_eq!(cmd.as_std().get_program().to_str().unwrap(), "docker");
    let args: Vec<&str> = cmd
        .as_std()
        .get_args()
        .map(|a| a.to_str().unwrap())
        .collect();
    assert!(args.contains(&"sandbox"));
    assert!(args.contains(&"run"));
    assert!(args.contains(&"sandbox-abc"));
    assert!(args.contains(&"--print"));
    assert!(args.contains(&"hello"));
}

#[test]
fn test_truncate_str_multibyte_utf8() {
    // Reproduces the panic from issue #35: slicing a string at a byte index
    // that falls inside a multi-byte UTF-8 character (e.g. '…' = 3 bytes).
    let mut s = "a".repeat(199);
    s.push('…'); // U+2026, 3 bytes → bytes 199..202
    s.push_str("trailing");
    assert!(s.len() > 200);

    // Before the fix, &s[..s.len().min(200)] panicked with
    // "byte index 200 is not a char boundary".
    let truncated = crate::truncate_str(&s, 200);
    assert_eq!(truncated.len(), 199); // stops before the '…'
    assert!(truncated.is_char_boundary(truncated.len()));
}

#[test]
fn test_truncate_str_ascii_only() {
    let s = "a".repeat(300);
    let truncated = crate::truncate_str(&s, 200);
    assert_eq!(truncated.len(), 200);
}

#[test]
fn test_truncate_str_short_string() {
    let s = "short";
    let truncated = crate::truncate_str(s, 200);
    assert_eq!(truncated, "short");
}

// --- ClaudeEventTranslator ---

mod translator {
    use super::super::ClaudeEventTranslator;
    use super::super::models::ClaudeEvent;
    use crate::output::Event;

    fn assistant_line(stop_reason: Option<&str>) -> ClaudeEvent {
        let reason_field = match stop_reason {
            Some(r) => format!(r#""{}""#, r),
            None => "null".to_string(),
        };
        let line = format!(
            r#"{{
                "type":"assistant",
                "message":{{
                    "model":"claude-sonnet-4-5",
                    "id":"msg_1",
                    "type":"message",
                    "role":"assistant",
                    "content":[{{"type":"text","text":"hi"}}],
                    "stop_reason":{},
                    "stop_sequence":null,
                    "usage":{{"input_tokens":10,"output_tokens":5}},
                    "context_management":null
                }},
                "parent_tool_use_id":null,
                "session_id":"s1",
                "uuid":"u1"
            }}"#,
            reason_field
        );
        serde_json::from_str(&line).unwrap()
    }

    fn result_line() -> ClaudeEvent {
        let line = r#"{
            "type":"result",
            "subtype":"success",
            "is_error":false,
            "duration_ms":1234,
            "duration_api_ms":1000,
            "num_turns":1,
            "result":"done",
            "session_id":"s1",
            "total_cost_usd":0.001,
            "usage":{"input_tokens":100,"output_tokens":50},
            "permission_denials":[],
            "uuid":"u2"
        }"#;
        serde_json::from_str(line).unwrap()
    }

    #[test]
    fn translates_assistant_to_single_assistant_message() {
        let mut translator = ClaudeEventTranslator::new();
        let events = translator.translate(&assistant_line(Some("end_turn")));
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], Event::AssistantMessage { .. }));
    }

    #[test]
    fn translates_result_to_turn_complete_followed_by_result() {
        let mut translator = ClaudeEventTranslator::new();
        // Feed an assistant message first so the translator captures
        // stop_reason + usage.
        let _ = translator.translate(&assistant_line(Some("end_turn")));
        let events = translator.translate(&result_line());
        assert_eq!(
            events.len(),
            2,
            "expected [TurnComplete, Result], got {:?}",
            events
        );
        match &events[0] {
            Event::TurnComplete {
                stop_reason,
                turn_index,
                usage,
            } => {
                assert_eq!(stop_reason.as_deref(), Some("end_turn"));
                assert_eq!(*turn_index, 0);
                assert!(usage.is_some(), "usage should be carried from assistant");
            }
            other => panic!("expected TurnComplete first, got {:?}", other),
        }
        assert!(matches!(events[1], Event::Result { .. }));
    }

    #[test]
    fn turn_index_is_monotonic_across_turns() {
        let mut translator = ClaudeEventTranslator::new();

        // Turn 0
        let _ = translator.translate(&assistant_line(Some("end_turn")));
        let turn0 = translator.translate(&result_line());
        match &turn0[0] {
            Event::TurnComplete { turn_index, .. } => assert_eq!(*turn_index, 0),
            other => panic!("expected TurnComplete, got {:?}", other),
        }

        // Turn 1
        let _ = translator.translate(&assistant_line(Some("tool_use")));
        let turn1 = translator.translate(&result_line());
        match &turn1[0] {
            Event::TurnComplete {
                turn_index,
                stop_reason,
                ..
            } => {
                assert_eq!(*turn_index, 1);
                assert_eq!(stop_reason.as_deref(), Some("tool_use"));
            }
            other => panic!("expected TurnComplete, got {:?}", other),
        }
    }

    #[test]
    fn stop_reason_is_none_when_assistant_did_not_surface_one() {
        let mut translator = ClaudeEventTranslator::new();
        let _ = translator.translate(&assistant_line(None));
        let events = translator.translate(&result_line());
        match &events[0] {
            Event::TurnComplete { stop_reason, .. } => assert!(stop_reason.is_none()),
            other => panic!("expected TurnComplete, got {:?}", other),
        }
    }

    #[test]
    fn stop_reason_is_cleared_between_turns() {
        let mut translator = ClaudeEventTranslator::new();

        // Turn 0: assistant reports end_turn.
        let _ = translator.translate(&assistant_line(Some("end_turn")));
        let _ = translator.translate(&result_line());

        // Turn 1: assistant reports null stop_reason, so TurnComplete
        // should carry None (not the stale "end_turn" from turn 0).
        let _ = translator.translate(&assistant_line(None));
        let events = translator.translate(&result_line());
        match &events[0] {
            Event::TurnComplete { stop_reason, .. } => {
                assert!(
                    stop_reason.is_none(),
                    "stop_reason should not leak across turns, got {:?}",
                    stop_reason
                );
            }
            other => panic!("expected TurnComplete, got {:?}", other),
        }
    }

    #[test]
    fn later_assistant_overwrites_earlier_stop_reason_within_turn() {
        let mut translator = ClaudeEventTranslator::new();
        // First assistant in the turn: tool_use (intermediate).
        let _ = translator.translate(&assistant_line(Some("tool_use")));
        // Second assistant in the turn: end_turn (final).
        let _ = translator.translate(&assistant_line(Some("end_turn")));
        let events = translator.translate(&result_line());
        match &events[0] {
            Event::TurnComplete { stop_reason, .. } => {
                assert_eq!(stop_reason.as_deref(), Some("end_turn"));
            }
            other => panic!("expected TurnComplete, got {:?}", other),
        }
    }
}
