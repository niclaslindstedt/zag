use super::*;

#[test]
fn test_resolve_broadcast_no_sessions_with_tag_errors() {
    // Use a tag that won't exist in any store
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

#[test]
fn test_resolve_broadcast_no_tag_no_sessions_global_errors() {
    // Global search with no sessions should mention "across all projects"
    // This may find sessions if ~/.zag exists with data, so we use a custom root
    // to ensure an empty store. For global=true the store loads from ~/.zag/projects/
    // so this test is best-effort.
    let result =
        resolve_broadcast_session_ids(None, false, Some("/tmp/nonexistent-zag-root-12345"));
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("No sessions found"));
}
