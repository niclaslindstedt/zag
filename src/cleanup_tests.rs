use super::*;

#[test]
fn test_print_resume_hint_without_provider_session_id() {
    // Should not panic, just prints to stdout
    print_resume_hint("wrapper-123", None, "Workspace");
}

#[test]
fn test_print_resume_hint_with_same_provider_session_id() {
    // When provider session ID matches wrapper, should not print native ID line
    print_resume_hint("wrapper-123", Some("wrapper-123"), "Sandbox");
}

#[test]
fn test_print_resume_hint_with_different_provider_session_id() {
    // When provider session ID differs, should print both
    print_resume_hint("wrapper-123", Some("native-456"), "Workspace");
}

#[test]
fn test_print_session_resume_hint_without_provider_id() {
    print_session_resume_hint("wrapper-123", None);
}

#[test]
fn test_print_session_resume_hint_with_provider_id() {
    print_session_resume_hint("wrapper-123", Some("native-456"));
}

#[test]
fn test_print_session_resume_hint_same_ids() {
    // When IDs match, should not print duplicate native ID
    print_session_resume_hint("wrapper-123", Some("wrapper-123"));
}
