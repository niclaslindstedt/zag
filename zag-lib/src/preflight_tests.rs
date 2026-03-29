use super::*;

#[test]
fn test_binary_for_agent_known() {
    assert_eq!(binary_for_agent("claude"), "claude");
    assert_eq!(binary_for_agent("codex"), "codex");
    assert_eq!(binary_for_agent("gemini"), "gemini");
    assert_eq!(binary_for_agent("copilot"), "copilot");
    assert_eq!(binary_for_agent("ollama"), "ollama");
}

#[test]
fn test_binary_for_agent_unknown_passthrough() {
    assert_eq!(binary_for_agent("custom-agent"), "custom-agent");
}

#[test]
fn test_find_in_path_finds_common_binary() {
    // "sh" should always exist on Unix systems
    #[cfg(unix)]
    {
        let result = find_in_path("sh");
        assert!(result.is_some(), "expected to find 'sh' in PATH");
    }
}

#[test]
fn test_find_in_path_returns_none_for_missing() {
    let result = find_in_path("zag-nonexistent-binary-xyz-123");
    assert!(result.is_none());
}

#[test]
fn test_check_binary_returns_error_for_missing() {
    let result = check_binary("zag-nonexistent-binary-xyz-123");
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("not found in PATH"));
}

#[test]
fn test_check_binary_error_contains_install_hint() {
    let result = check_binary("claude");
    // This may succeed or fail depending on whether claude is installed,
    // but if it fails, the error should contain install instructions
    if let Err(e) = result {
        assert!(e.to_string().contains("Install:"));
    }
}

#[test]
fn test_install_hint_known_agents() {
    assert!(install_hint("claude").contains("npm"));
    assert!(install_hint("codex").contains("npm"));
    assert!(install_hint("ollama").contains("ollama.ai"));
}
