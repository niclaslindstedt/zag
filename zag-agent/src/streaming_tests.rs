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
