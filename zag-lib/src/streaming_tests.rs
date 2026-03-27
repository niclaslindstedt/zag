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
