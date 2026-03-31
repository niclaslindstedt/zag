use super::*;

#[test]
fn test_session_status_display() {
    assert_eq!(SessionStatus::Running.to_string(), "running");
    assert_eq!(SessionStatus::Idle.to_string(), "idle");
    assert_eq!(SessionStatus::Completed.to_string(), "completed");
    assert_eq!(SessionStatus::Failed.to_string(), "failed");
    assert_eq!(SessionStatus::Dead.to_string(), "dead");
    assert_eq!(SessionStatus::Unknown.to_string(), "unknown");
}

#[test]
fn test_is_recent_true() {
    let ts = chrono::Utc::now().to_rfc3339();
    assert!(is_recent(&ts, 30));
}

#[test]
fn test_is_recent_false() {
    let old = chrono::Utc::now() - chrono::Duration::seconds(60);
    let ts = old.to_rfc3339();
    assert!(!is_recent(&ts, 30));
}

#[test]
fn test_is_recent_invalid_ts() {
    assert!(!is_recent("not-a-timestamp", 30));
}
