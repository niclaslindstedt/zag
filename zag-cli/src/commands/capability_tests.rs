use super::*;

#[test]
fn get_capability_claude() {
    let cap = get_capability("claude").unwrap();
    assert_eq!(cap.provider, "claude");
    assert_eq!(cap.default_model, crate::claude::DEFAULT_MODEL);
    assert_eq!(
        cap.available_models,
        models_to_vec(crate::claude::AVAILABLE_MODELS)
    );
    assert!(cap.features.resume.supported);
    assert!(cap.features.resume.native);
    assert!(cap.features.json_schema.supported);
    assert!(cap.features.json_schema.native);
    assert!(cap.features.stream_json.supported);
    assert!(!cap.features.review.supported);
    assert!(cap.features.worktree.supported);
    assert!(!cap.features.worktree.native);
}

#[test]
fn get_capability_codex() {
    let cap = get_capability("codex").unwrap();
    assert_eq!(cap.provider, "codex");
    assert_eq!(cap.default_model, crate::codex::DEFAULT_MODEL);
    assert!(cap.features.review.supported);
    assert!(cap.features.review.native);
    assert!(cap.features.json_schema.supported);
    assert!(!cap.features.json_schema.native);
    assert_eq!(
        cap.features.session_logs.completeness,
        Some("partial".to_string())
    );
}

#[test]
fn get_capability_gemini() {
    let cap = get_capability("gemini").unwrap();
    assert_eq!(cap.provider, "gemini");
    assert_eq!(cap.default_model, crate::gemini::DEFAULT_MODEL);
    assert!(cap.features.resume.supported);
    assert!(!cap.features.resume_with_prompt.supported);
    assert!(cap.features.json_output.supported);
    assert!(!cap.features.json_output.native);
    assert_eq!(
        cap.features.session_logs.completeness,
        Some("full".to_string())
    );
}

#[test]
fn get_capability_copilot() {
    let cap = get_capability("copilot").unwrap();
    assert_eq!(cap.provider, "copilot");
    assert_eq!(cap.default_model, crate::copilot::DEFAULT_MODEL);
    assert!(!cap.features.json_output.supported);
    assert!(!cap.features.stream_json.supported);
    assert!(!cap.features.resume_with_prompt.supported);
    assert!(cap.features.system_prompt.supported);
    assert!(!cap.features.system_prompt.native);
}

#[test]
fn get_capability_ollama() {
    let cap = get_capability("ollama").unwrap();
    assert_eq!(cap.provider, "ollama");
    assert_eq!(cap.default_model, crate::ollama::DEFAULT_MODEL);
    assert!(!cap.features.resume.supported);
    assert!(!cap.features.session_logs.supported);
    assert!(cap.features.session_logs.completeness.is_none());
    assert!(!cap.features.add_dirs.supported);
    assert!(cap.features.json_output.supported);
    assert!(!cap.features.json_output.native);
}

#[test]
fn get_capability_unknown_provider() {
    let result = get_capability("unknown");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("No capabilities defined")
    );
}

#[test]
fn format_json_compact() {
    let cap = get_capability("claude").unwrap();
    let output = format_capability(&cap, "json", false).unwrap();
    assert!(!output.contains('\n'));
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["provider"], "claude");
}

#[test]
fn format_json_pretty() {
    let cap = get_capability("claude").unwrap();
    let output = format_capability(&cap, "json", true).unwrap();
    assert!(output.contains('\n'));
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["provider"], "claude");
}

#[test]
fn format_yaml() {
    let cap = get_capability("gemini").unwrap();
    let output = format_capability(&cap, "yaml", false).unwrap();
    assert!(output.contains("provider: gemini"));
}

#[test]
fn format_toml() {
    let cap = get_capability("codex").unwrap();
    let output = format_capability(&cap, "toml", false).unwrap();
    assert!(output.contains("provider = \"codex\""));
}

#[test]
fn format_unsupported() {
    let cap = get_capability("claude").unwrap();
    let result = format_capability(&cap, "xml", false);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Unsupported format")
    );
}

#[test]
fn all_providers_have_size_mappings() {
    for provider in &["claude", "codex", "gemini", "copilot", "ollama"] {
        let cap = get_capability(provider).unwrap();
        assert!(!cap.size_mappings.small.is_empty(), "{} small", provider);
        assert!(!cap.size_mappings.medium.is_empty(), "{} medium", provider);
        assert!(!cap.size_mappings.large.is_empty(), "{} large", provider);
    }
}

#[test]
fn all_providers_support_interactive_and_non_interactive() {
    for provider in &["claude", "codex", "gemini", "copilot", "ollama"] {
        let cap = get_capability(provider).unwrap();
        assert!(
            cap.features.interactive.supported,
            "{} interactive",
            provider
        );
        assert!(
            cap.features.non_interactive.supported,
            "{} non_interactive",
            provider
        );
    }
}

#[test]
fn worktree_and_sandbox_are_wrapper_for_all() {
    for provider in &["claude", "codex", "gemini", "copilot", "ollama"] {
        let cap = get_capability(provider).unwrap();
        assert!(cap.features.worktree.supported, "{} worktree", provider);
        assert!(
            !cap.features.worktree.native,
            "{} worktree native",
            provider
        );
        assert!(cap.features.sandbox.supported, "{} sandbox", provider);
        assert!(!cap.features.sandbox.native, "{} sandbox native", provider);
    }
}

#[test]
fn session_logs_completeness_absent_when_unsupported() {
    let cap = get_capability("ollama").unwrap();
    assert!(!cap.features.session_logs.supported);
    // Verify completeness is None and thus skipped in serialization
    let json = serde_json::to_string(&cap.features.session_logs).unwrap();
    assert!(!json.contains("completeness"));
}
