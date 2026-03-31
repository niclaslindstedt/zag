use super::*;

#[test]
fn test_resolve_broadcast_no_sessions_errors() {
    // Use a tag that won't exist in any store
    let result = resolve_broadcast_session_ids("nonexistent-tag-12345", false, Some("/tmp"));
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("No sessions found with tag"));
    assert!(err.contains("nonexistent-tag-12345"));
}
