//! The resolve + send logic now lives in `zag-orch/src/messaging.rs`; these
//! tests just confirm the library helper is reachable from the CLI module.

use zag_orch::messaging::resolve_broadcast_session_ids;

#[test]
fn test_resolve_broadcast_no_sessions_with_tag_errors() {
    let result = resolve_broadcast_session_ids(Some("nonexistent-tag-12345"), false, Some("/tmp"));
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("No sessions found with tag"));
    assert!(err.contains("nonexistent-tag-12345"));
}

#[test]
fn test_resolve_broadcast_no_tag_no_sessions_errors() {
    let result = resolve_broadcast_session_ids(None, false, Some("/tmp"));
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("No sessions found in current project"));
}
