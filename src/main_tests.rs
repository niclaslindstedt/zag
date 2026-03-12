use super::*;

#[test]
fn test_capitalize_normal() {
    assert_eq!(capitalize("claude"), "Claude");
    assert_eq!(capitalize("codex"), "Codex");
    assert_eq!(capitalize("gemini"), "Gemini");
}

#[test]
fn test_capitalize_already_capitalized() {
    assert_eq!(capitalize("Claude"), "Claude");
}

#[test]
fn test_capitalize_empty() {
    assert_eq!(capitalize(""), "");
}

#[test]
fn test_capitalize_single_char() {
    assert_eq!(capitalize("a"), "A");
}

#[test]
fn test_resolve_provider_from_flag() {
    assert_eq!(resolve_provider(Some("claude"), None).unwrap(), "claude");
    assert_eq!(resolve_provider(Some("codex"), None).unwrap(), "codex");
    assert_eq!(resolve_provider(Some("gemini"), None).unwrap(), "gemini");
    assert_eq!(resolve_provider(Some("copilot"), None).unwrap(), "copilot");
}

#[test]
fn test_resolve_provider_case_insensitive() {
    assert_eq!(resolve_provider(Some("CLAUDE"), None).unwrap(), "claude");
    assert_eq!(resolve_provider(Some("Gemini"), None).unwrap(), "gemini");
}

#[test]
fn test_resolve_provider_invalid() {
    let result = resolve_provider(Some("invalid"), None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid provider"));
}

#[test]
fn test_resolve_provider_default() {
    // When no flag and no config, defaults to "claude"
    let result = resolve_provider(None, None).unwrap();
    assert_eq!(result, "claude");
}
