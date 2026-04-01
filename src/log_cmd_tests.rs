use super::*;

#[test]
fn test_validate_level_valid() {
    assert!(validate_level("info").is_ok());
    assert!(validate_level("warn").is_ok());
    assert!(validate_level("error").is_ok());
    assert!(validate_level("debug").is_ok());
}

#[test]
fn test_validate_level_invalid() {
    assert!(validate_level("critical").is_err());
    assert!(validate_level("INFO").is_err());
    assert!(validate_level("").is_err());
}

#[test]
fn test_resolve_session_id_from_flag() {
    let result = resolve_session_id(Some("abc-123"));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "abc-123");
}

#[test]
fn test_resolve_session_id_missing() {
    // Unset ZAG_SESSION_ID to ensure it's not set
    unsafe { std::env::remove_var("ZAG_SESSION_ID") };
    let result = resolve_session_id(None);
    assert!(result.is_err());
}
