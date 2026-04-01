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

#[test]
fn test_is_recent_with_heartbeat_timestamp() {
    // A recent heartbeat should indicate running
    let recent_ts = chrono::Utc::now().to_rfc3339();
    assert!(is_recent(&recent_ts, 30));

    // An old heartbeat should indicate idle
    let old_ts = (chrono::Utc::now() - chrono::Duration::seconds(60)).to_rfc3339();
    assert!(!is_recent(&old_ts, 30));
}

#[test]
fn test_status_info_serialization() {
    let info = StatusInfo {
        session_id: "test-123".to_string(),
        status: SessionStatus::Running,
        provider: "claude".to_string(),
        model: "opus".to_string(),
        name: Some("my-session".to_string()),
        pid: Some(12345),
        error: None,
        last_heartbeat_at: Some("2024-01-01T00:00:00Z".to_string()),
    };
    let json = serde_json::to_string(&info).unwrap();
    assert!(json.contains("\"last_heartbeat_at\""));
    assert!(json.contains("2024-01-01T00:00:00Z"));

    // Without heartbeat, field should be absent
    let info_no_hb = StatusInfo {
        last_heartbeat_at: None,
        ..info
    };
    let json2 = serde_json::to_string(&info_no_hb).unwrap();
    assert!(!json2.contains("last_heartbeat_at"));
}
