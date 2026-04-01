use super::*;

#[test]
fn test_extract_prompt_missing_session() {
    // Non-existent session should return None
    let result = extract_prompt("nonexistent-session-id", Some("/nonexistent"));
    assert!(result.is_none());
}

#[test]
fn test_run_retry_no_sessions() {
    let params = RetryParams {
        session_ids: vec![],
        tag: None,
        failed: false,
        model: None,
        json: false,
        root: Some("/nonexistent".to_string()),
    };
    let result = run_retry(params);
    assert!(result.is_err());
}

#[test]
fn test_run_retry_session_not_found() {
    let params = RetryParams {
        session_ids: vec!["nonexistent-id".to_string()],
        tag: None,
        failed: false,
        model: None,
        json: false,
        root: Some("/nonexistent".to_string()),
    };
    // Should succeed but report error per-session (not bail)
    let result = run_retry(params);
    assert!(result.is_ok());
}
