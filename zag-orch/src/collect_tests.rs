use super::*;

#[test]
fn test_collected_result_serialization() {
    let result = CollectedResult {
        session_id: "abc-123".to_string(),
        name: Some("test".to_string()),
        provider: "claude".to_string(),
        model: "sonnet".to_string(),
        status: "completed".to_string(),
        result_text: Some("Hello world".to_string()),
        error: None,
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("abc-123"));
    assert!(json.contains("completed"));
    assert!(!json.contains("error")); // skip_serializing_if = "Option::is_none"
}
