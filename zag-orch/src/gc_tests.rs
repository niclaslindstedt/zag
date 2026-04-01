use super::*;

#[test]
fn parse_duration_days() {
    assert_eq!(parse_duration_secs("7d").unwrap(), 7 * 86400);
    assert_eq!(parse_duration_secs("30d").unwrap(), 30 * 86400);
}

#[test]
fn parse_duration_hours() {
    assert_eq!(parse_duration_secs("24h").unwrap(), 24 * 3600);
    assert_eq!(parse_duration_secs("1h").unwrap(), 3600);
}

#[test]
fn parse_duration_invalid() {
    assert!(parse_duration_secs("abc").is_err());
    assert!(parse_duration_secs("7m").is_err());
    assert!(parse_duration_secs("").is_err());
}

#[test]
fn is_file_old_returns_false_for_recent_file() {
    let dir = std::env::temp_dir().join(format!("zag-gc-test-new-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("new.txt");
    std::fs::write(&path, "data").unwrap();
    // A just-created file should not be old
    let cutoff = std::time::SystemTime::now() - std::time::Duration::from_secs(7 * 86400);
    assert!(!is_file_old(&path, cutoff));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn is_file_old_nonexistent_file() {
    let path = std::env::temp_dir().join("zag-gc-nonexistent-12345.txt");
    let cutoff = std::time::SystemTime::now() - std::time::Duration::from_secs(7 * 86400);
    assert!(!is_file_old(&path, cutoff));
}

#[test]
fn has_session_ended_detects_ended() {
    let dir = std::env::temp_dir().join(format!("zag-gc-test-ended-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("test.jsonl");
    let content = r#"{"seq":1,"ts":"2024-01-01T00:00:00Z","source":"wrapper","wrapper_session_id":"s1","kind":{"type":"session_started","provider":"claude","command":"run"}}
{"seq":2,"ts":"2024-01-01T00:01:00Z","source":"wrapper","wrapper_session_id":"s1","kind":{"type":"SessionEnded","success":true}}"#;
    std::fs::write(&path, content).unwrap();
    assert!(has_session_ended(&path));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn has_session_ended_returns_false_for_running() {
    let dir = std::env::temp_dir().join(format!("zag-gc-test-running-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("test.jsonl");
    let content = r#"{"seq":1,"ts":"2024-01-01T00:00:00Z","source":"wrapper","wrapper_session_id":"s1","kind":{"type":"session_started","provider":"claude","command":"run"}}"#;
    std::fs::write(&path, content).unwrap();
    assert!(!has_session_ended(&path));
    let _ = std::fs::remove_dir_all(&dir);
}
