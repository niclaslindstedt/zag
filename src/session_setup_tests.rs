use super::*;

#[test]
fn test_setup_plain_session_explicit_id() {
    let result = setup_plain_session(false, &None, &Some("my-session-id".to_string()));
    assert_eq!(result.session_id.as_deref(), Some("my-session-id"));
    assert!(result.workspace_path.is_some());
}

#[test]
fn test_setup_plain_session_non_interactive_no_id() {
    let result = setup_plain_session(false, &None, &None);
    assert!(result.session_id.is_none());
    assert!(result.workspace_path.is_none());
}

#[test]
fn test_setup_plain_session_interactive_generates_uuid() {
    let result = setup_plain_session(true, &None, &None);
    assert!(result.session_id.is_some());
    let id = result.session_id.unwrap();
    // Should be a valid UUID format (36 chars with hyphens)
    assert_eq!(id.len(), 36);
    assert!(result.workspace_path.is_some());
}

#[test]
fn test_setup_plain_session_interactive_unique_ids() {
    let result1 = setup_plain_session(true, &None, &None);
    let result2 = setup_plain_session(true, &None, &None);
    assert_ne!(result1.session_id, result2.session_id);
}
