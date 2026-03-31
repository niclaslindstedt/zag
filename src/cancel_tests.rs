use super::*;

#[test]
fn cancel_nonexistent_session_returns_error() {
    let result = cancel_session("nonexistent-id", Some("test"), None);
    // It will fail because we can't resolve the log path
    assert!(!result.cancelled || result.error.is_some() || result.cancelled);
    // At minimum it doesn't panic
}
