use super::*;

#[test]
fn test_send_user_message_format() {
    // Verify the JSON format produced by send_user_message
    let msg = serde_json::json!({
        "type": "user_message",
        "content": "hello world",
    });
    let serialized = serde_json::to_string(&msg).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    assert_eq!(parsed["type"], "user_message");
    assert_eq!(parsed["content"], "hello world");
}

#[test]
fn test_send_user_message_escapes_special_chars() {
    let msg = serde_json::json!({
        "type": "user_message",
        "content": "line1\nline2\ttab\"quote",
    });
    let serialized = serde_json::to_string(&msg).unwrap();
    // Should be valid single-line JSON (no raw newlines)
    assert!(!serialized.contains('\n'));
    let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    assert_eq!(parsed["content"], "line1\nline2\ttab\"quote");
}

// ---------------------------------------------------------------------------
// Tests using real child processes
// ---------------------------------------------------------------------------

use tokio::process::Command;

/// Spawn a child process with piped stdin/stdout for testing StreamingSession.
fn spawn_echo_child() -> tokio::process::Child {
    Command::new("cat")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn cat process")
}

#[tokio::test]
async fn test_streaming_session_new_valid_child() {
    let child = spawn_echo_child();
    let session = StreamingSession::new(child);
    assert!(session.is_ok());
}

#[tokio::test]
async fn test_streaming_session_new_no_stdout() {
    // Spawn without piped stdout
    let child = Command::new("true")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn");
    let result = StreamingSession::new(child);
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(err.to_string().contains("stdout not piped"));
}

#[tokio::test]
async fn test_next_event_returns_none_on_eof() {
    // Spawn a process that immediately exits (empty stdout)
    let child = Command::new("true")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn");
    let mut session = StreamingSession::new(child).unwrap();
    let event = session.next_event().await.unwrap();
    assert!(event.is_none());
}

#[tokio::test]
async fn test_next_event_skips_empty_lines() {
    // Echo some empty lines then close
    let child = Command::new("printf")
        .arg("\n\n\n")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn");
    let mut session = StreamingSession::new(child).unwrap();
    let event = session.next_event().await.unwrap();
    assert!(event.is_none()); // All lines are empty, so we get None at EOF
}

#[tokio::test]
async fn test_next_event_skips_unparseable_json() {
    // Echo invalid JSON followed by EOF
    let child = Command::new("printf")
        .arg("not json\nalso not json\n")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn");
    let mut session = StreamingSession::new(child).unwrap();
    let event = session.next_event().await.unwrap();
    assert!(event.is_none()); // Skips both lines, returns None at EOF
}

#[tokio::test]
async fn test_close_input() {
    let child = spawn_echo_child();
    let mut session = StreamingSession::new(child).unwrap();
    assert!(session.stdin.is_some());
    session.close_input();
    assert!(session.stdin.is_none());
}

#[tokio::test]
async fn test_send_after_close_fails() {
    let child = spawn_echo_child();
    let mut session = StreamingSession::new(child).unwrap();
    session.close_input();
    let result = session.send("test").await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("stdin already closed")
    );
}

#[tokio::test]
async fn test_wait_success() {
    let child = Command::new("true")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn");
    let session = StreamingSession::new(child).unwrap();
    let result = session.wait().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_wait_failure() {
    let child = Command::new("false")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn");
    let session = StreamingSession::new(child).unwrap();
    let result = session.wait().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("failed"));
}

// ---------------------------------------------------------------------------
// Tests for Claude stream-json parsing (bidirectional streaming)
// ---------------------------------------------------------------------------

use crate::output::{ContentBlock, Event};

/// Spawn a `sh -c` child whose stdout is a fixed sequence of JSON lines.
/// JSON lines must not contain single quotes (they don't in Claude output).
fn spawn_with_jsonl(lines: &[&str]) -> tokio::process::Child {
    // Join with literal newlines between each single-quoted arg; printf handles
    // the trailing newline per entry.
    let joined = lines
        .iter()
        .map(|l| format!("'{}'", l))
        .collect::<Vec<_>>()
        .join(" ");
    let script = format!("printf '%s\\n' {}", joined);
    Command::new("sh")
        .arg("-c")
        .arg(script)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn sh")
}

/// Build a minimal Claude `assistant` event with a single text block.
fn claude_assistant_text_line(text: &str) -> String {
    claude_assistant_text_line_with_stop(text, None)
}

/// Build a minimal Claude `assistant` event with a text block and an
/// explicit `stop_reason`.
fn claude_assistant_text_line_with_stop(text: &str, stop_reason: Option<&str>) -> String {
    let reason = match stop_reason {
        Some(r) => format!(r#""{}""#, r),
        None => "null".to_string(),
    };
    format!(
        r#"{{"type":"assistant","message":{{"model":"claude-sonnet-4-5","id":"msg_1","type":"message","role":"assistant","content":[{{"type":"text","text":"{}"}}],"stop_reason":{},"stop_sequence":null,"usage":{{"input_tokens":10,"output_tokens":5}},"context_management":null}},"parent_tool_use_id":null,"session_id":"s1","uuid":"u1"}}"#,
        text, reason
    )
}

/// Build a minimal Claude `result` event for an agent turn.
fn claude_result_line(turns: u32) -> String {
    format!(
        r#"{{"type":"result","subtype":"success","is_error":false,"duration_ms":1234,"duration_api_ms":1000,"num_turns":{},"result":"done","session_id":"s1","total_cost_usd":0.01,"usage":{{"input_tokens":10,"output_tokens":5}},"uuid":"u2"}}"#,
        turns
    )
}

#[tokio::test]
async fn test_next_event_parses_claude_assistant_message() {
    let line = claude_assistant_text_line("hello");
    let child = spawn_with_jsonl(&[&line]);
    let mut session = StreamingSession::new(child).unwrap();

    let event = session.next_event().await.unwrap();
    match event {
        Some(Event::AssistantMessage { content, .. }) => {
            assert_eq!(content.len(), 1);
            match &content[0] {
                ContentBlock::Text { text } => assert_eq!(text, "hello"),
                other => panic!("expected text block, got {:?}", other),
            }
        }
        other => panic!("expected AssistantMessage, got {:?}", other),
    }

    // EOF after the one line.
    assert!(session.next_event().await.unwrap().is_none());
}

#[tokio::test]
async fn test_next_event_parses_claude_result_per_turn() {
    // Two agent turns, each an assistant message followed by a result event.
    // Each turn produces three unified events: AssistantMessage,
    // TurnComplete (synthesized), and Result.
    let turn1_assistant = claude_assistant_text_line("first answer");
    let turn1_result = claude_result_line(1);
    let turn2_assistant = claude_assistant_text_line("second answer");
    let turn2_result = claude_result_line(2);

    let child = spawn_with_jsonl(&[
        &turn1_assistant,
        &turn1_result,
        &turn2_assistant,
        &turn2_result,
    ]);
    let mut session = StreamingSession::new(child).unwrap();

    let mut events = Vec::new();
    while let Some(event) = session.next_event().await.unwrap() {
        events.push(event);
    }

    assert_eq!(
        events.len(),
        6,
        "expected 6 unified events (2 * [AssistantMessage, TurnComplete, Result]), got {:?}",
        events
    );
    assert!(matches!(events[0], Event::AssistantMessage { .. }));
    assert!(
        matches!(events[1], Event::TurnComplete { turn_index: 0, .. }),
        "expected turn-1 TurnComplete, got {:?}",
        events[1]
    );
    assert!(
        matches!(
            events[2],
            Event::Result {
                success: true,
                num_turns: Some(1),
                ..
            }
        ),
        "expected turn-1 Result, got {:?}",
        events[2]
    );
    assert!(matches!(events[3], Event::AssistantMessage { .. }));
    assert!(
        matches!(events[4], Event::TurnComplete { turn_index: 1, .. }),
        "expected turn-2 TurnComplete, got {:?}",
        events[4]
    );
    assert!(
        matches!(
            events[5],
            Event::Result {
                success: true,
                num_turns: Some(2),
                ..
            }
        ),
        "expected turn-2 Result, got {:?}",
        events[5]
    );
}

#[tokio::test]
async fn test_next_event_turn_complete_carries_stop_reason_and_index() {
    // Three turns, each with a distinct stop_reason, verifying that
    // TurnComplete carries the correct stop_reason per turn and that
    // turn_index is monotonic starting at 0.
    let turn1_assistant = claude_assistant_text_line_with_stop("one", Some("end_turn"));
    let turn1_result = claude_result_line(1);
    let turn2_assistant = claude_assistant_text_line_with_stop("two", Some("tool_use"));
    let turn2_result = claude_result_line(2);
    let turn3_assistant = claude_assistant_text_line_with_stop("three", Some("max_tokens"));
    let turn3_result = claude_result_line(3);

    let child = spawn_with_jsonl(&[
        &turn1_assistant,
        &turn1_result,
        &turn2_assistant,
        &turn2_result,
        &turn3_assistant,
        &turn3_result,
    ]);
    let mut session = StreamingSession::new(child).unwrap();

    let mut events = Vec::new();
    while let Some(event) = session.next_event().await.unwrap() {
        events.push(event);
    }

    // 3 turns * 3 events (AssistantMessage, TurnComplete, Result) = 9.
    assert_eq!(
        events.len(),
        9,
        "expected 9 unified events, got {:?}",
        events
    );

    // Turn 0.
    assert!(matches!(events[0], Event::AssistantMessage { .. }));
    match &events[1] {
        Event::TurnComplete {
            stop_reason,
            turn_index,
            usage,
        } => {
            assert_eq!(stop_reason.as_deref(), Some("end_turn"));
            assert_eq!(*turn_index, 0);
            assert!(usage.is_some());
        }
        other => panic!("expected TurnComplete for turn 0, got {:?}", other),
    }
    assert!(matches!(events[2], Event::Result { .. }));

    // Turn 1.
    assert!(matches!(events[3], Event::AssistantMessage { .. }));
    match &events[4] {
        Event::TurnComplete {
            stop_reason,
            turn_index,
            ..
        } => {
            assert_eq!(stop_reason.as_deref(), Some("tool_use"));
            assert_eq!(*turn_index, 1);
        }
        other => panic!("expected TurnComplete for turn 1, got {:?}", other),
    }
    assert!(matches!(events[5], Event::Result { .. }));

    // Turn 2.
    assert!(matches!(events[6], Event::AssistantMessage { .. }));
    match &events[7] {
        Event::TurnComplete {
            stop_reason,
            turn_index,
            ..
        } => {
            assert_eq!(stop_reason.as_deref(), Some("max_tokens"));
            assert_eq!(*turn_index, 2);
        }
        other => panic!("expected TurnComplete for turn 2, got {:?}", other),
    }
    assert!(matches!(events[8], Event::Result { .. }));
}

#[tokio::test]
async fn test_next_event_turn_complete_fires_before_result() {
    // Regression: the TurnComplete event must always precede the Result
    // event for the same turn, even though both are produced from a
    // single Claude `result` event.
    let assistant = claude_assistant_text_line_with_stop("hi", Some("end_turn"));
    let result = claude_result_line(1);
    let child = spawn_with_jsonl(&[&assistant, &result]);
    let mut session = StreamingSession::new(child).unwrap();

    let e0 = session.next_event().await.unwrap().unwrap();
    assert!(matches!(e0, Event::AssistantMessage { .. }));

    let e1 = session.next_event().await.unwrap().unwrap();
    assert!(
        matches!(e1, Event::TurnComplete { .. }),
        "expected TurnComplete before Result, got {:?}",
        e1
    );

    let e2 = session.next_event().await.unwrap().unwrap();
    assert!(
        matches!(e2, Event::Result { .. }),
        "expected Result after TurnComplete, got {:?}",
        e2
    );

    assert!(session.next_event().await.unwrap().is_none());
}

#[tokio::test]
async fn test_next_event_skips_thinking_blocks() {
    // An assistant message whose only content block is `thinking` should be
    // filtered down to an AssistantMessage with empty content — but still
    // emitted. We want to assert that the thinking block itself is stripped.
    let line = r#"{"type":"assistant","message":{"model":"claude-sonnet-4-5","id":"msg_1","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"internal reasoning"}],"stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":10,"output_tokens":5},"context_management":null},"parent_tool_use_id":null,"session_id":"s1","uuid":"u1"}"#;
    let child = spawn_with_jsonl(&[line]);
    let mut session = StreamingSession::new(child).unwrap();

    let event = session.next_event().await.unwrap();
    match event {
        Some(Event::AssistantMessage { content, .. }) => {
            assert!(
                content.is_empty(),
                "thinking block should be stripped, got {:?}",
                content
            );
        }
        other => panic!("expected AssistantMessage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_next_event_skips_unknown_claude_events() {
    // Unknown event types become ClaudeEvent::Other and should be skipped
    // transparently rather than surfaced. Feed one unknown event followed by
    // a real assistant message; next_event should return the assistant.
    let unknown = r#"{"type":"rate_limit_event","foo":"bar"}"#;
    let known = claude_assistant_text_line("after unknown");
    let child = spawn_with_jsonl(&[unknown, &known]);
    let mut session = StreamingSession::new(child).unwrap();

    let event = session.next_event().await.unwrap();
    assert!(
        matches!(event, Some(Event::AssistantMessage { .. })),
        "expected AssistantMessage after skipping unknown event, got {:?}",
        event
    );
}
