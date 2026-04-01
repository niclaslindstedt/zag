use super::*;

#[test]
fn format_duration_seconds() {
    assert_eq!(format_duration(45.0), "45s");
}

#[test]
fn format_duration_minutes() {
    assert_eq!(format_duration(154.0), "2m 34s");
}

#[test]
fn format_duration_hours() {
    assert_eq!(format_duration(3720.0), "1h 2m");
}

#[test]
fn session_summary_includes_usage_fields() {
    let summary = SessionSummary {
        session_id: "test-1".to_string(),
        name: None,
        provider: "claude".to_string(),
        model: "opus".to_string(),
        status: "completed".to_string(),
        duration_secs: Some(120.0),
        turns: 5,
        tool_calls: HashMap::new(),
        total_tool_calls: 0,
        files_modified: vec![],
        result: None,
        error: None,
        event_count: 10,
        input_tokens: Some(5000),
        output_tokens: Some(1500),
        total_cost_usd: Some(0.025),
    };
    let json = serde_json::to_string(&summary).unwrap();
    assert!(json.contains("\"input_tokens\":5000"));
    assert!(json.contains("\"output_tokens\":1500"));
    assert!(json.contains("0.025"));
}

#[test]
fn session_summary_omits_usage_when_absent() {
    let summary = SessionSummary {
        session_id: "test-2".to_string(),
        name: None,
        provider: "claude".to_string(),
        model: "opus".to_string(),
        status: "completed".to_string(),
        duration_secs: None,
        turns: 1,
        tool_calls: HashMap::new(),
        total_tool_calls: 0,
        files_modified: vec![],
        result: None,
        error: None,
        event_count: 2,
        input_tokens: None,
        output_tokens: None,
        total_cost_usd: None,
    };
    let json = serde_json::to_string(&summary).unwrap();
    assert!(!json.contains("input_tokens"));
    assert!(!json.contains("output_tokens"));
    assert!(!json.contains("total_cost_usd"));
}
