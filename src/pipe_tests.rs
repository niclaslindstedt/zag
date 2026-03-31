use super::*;

#[test]
fn resolve_pipe_sessions_with_explicit_ids() {
    let ids = vec!["abc".to_string(), "def".to_string()];
    let result = resolve_pipe_sessions(&ids, None, None).unwrap();
    assert_eq!(result, ids);
}

#[test]
fn resolve_pipe_sessions_empty_errors() {
    let result = resolve_pipe_sessions(&[], None, None);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("No sessions specified")
    );
}

#[test]
fn build_context_single_session_no_index() {
    // Can't resolve a nonexistent session — returns None from extract
    let result = build_context(&["nonexistent".to_string()], None);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("No results available")
    );
}
