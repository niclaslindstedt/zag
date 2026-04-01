use super::*;

#[test]
fn test_resolve_session_ids_with_explicit_id() {
    let params = OutputParams {
        session_id: Some("test-123".to_string()),
        latest: false,
        output_name: None,
        tag: None,
        json: false,
        root: Some("/nonexistent".to_string()),
    };
    let ids = resolve_session_ids(&params).unwrap();
    assert_eq!(ids, vec!["test-123".to_string()]);
}

#[test]
fn test_resolve_session_ids_no_sessions() {
    let params = OutputParams {
        session_id: None,
        latest: true,
        output_name: None,
        tag: None,
        json: false,
        root: Some("/nonexistent".to_string()),
    };
    let result = resolve_session_ids(&params);
    assert!(result.is_err());
}
