use super::*;

#[test]
fn test_whoami_info_is_inside_session_with_session_id() {
    let info = WhoamiInfo {
        session_id: Some("test-123".to_string()),
        session_name: None,
        process_id: None,
        pid: 1234,
        provider: None,
        model: None,
        root: None,
        parent_session_id: None,
        parent_process_id: None,
    };
    assert!(info.is_inside_session());
}

#[test]
fn test_whoami_info_is_inside_session_with_process_id() {
    let info = WhoamiInfo {
        session_id: None,
        session_name: None,
        process_id: Some("proc-456".to_string()),
        pid: 1234,
        provider: None,
        model: None,
        root: None,
        parent_session_id: None,
        parent_process_id: None,
    };
    assert!(info.is_inside_session());
}

#[test]
fn test_whoami_info_is_not_inside_session() {
    let info = WhoamiInfo {
        session_id: None,
        session_name: None,
        process_id: None,
        pid: 1234,
        provider: None,
        model: None,
        root: None,
        parent_session_id: None,
        parent_process_id: None,
    };
    assert!(!info.is_inside_session());
}

#[test]
fn test_whoami_info_serialization() {
    let info = WhoamiInfo {
        session_id: Some("s-1".to_string()),
        session_name: Some("my-agent".to_string()),
        process_id: Some("p-1".to_string()),
        pid: 42,
        provider: Some("claude".to_string()),
        model: Some("opus".to_string()),
        root: Some("/tmp".to_string()),
        parent_session_id: None,
        parent_process_id: None,
    };
    let json = serde_json::to_string(&info).unwrap();
    assert!(json.contains("\"session_id\":\"s-1\""));
    assert!(json.contains("\"provider\":\"claude\""));
    assert!(json.contains("\"pid\":42"));
}
