use super::*;

fn make_test_capability() -> ProviderCapability {
    ProviderCapability {
        provider: "test".to_string(),
        default_model: "test-model".to_string(),
        available_models: vec!["test-model".to_string(), "test-small".to_string()],
        size_mappings: SizeMappings {
            small: "test-small".to_string(),
            medium: "test-model".to_string(),
            large: "test-large".to_string(),
        },
        features: Features {
            interactive: FeatureSupport::native(),
            non_interactive: FeatureSupport::native(),
            resume: FeatureSupport::native(),
            resume_with_prompt: FeatureSupport::unsupported(),
            session_logs: SessionLogSupport::full(),
            json_output: FeatureSupport::wrapper(),
            stream_json: FeatureSupport::unsupported(),
            json_schema: FeatureSupport::wrapper(),
            input_format: FeatureSupport::unsupported(),
            streaming_input: FeatureSupport::unsupported(),
            worktree: FeatureSupport::wrapper(),
            sandbox: FeatureSupport::wrapper(),
            system_prompt: FeatureSupport::native(),
            auto_approve: FeatureSupport::native(),
            review: FeatureSupport::unsupported(),
            add_dirs: FeatureSupport::native(),
            max_turns: FeatureSupport::native(),
        },
    }
}

#[test]
fn format_json_compact() {
    let cap = make_test_capability();
    let output = format_capability(&cap, "json", false).unwrap();
    assert!(!output.contains('\n'));
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["provider"], "test");
}

#[test]
fn format_json_pretty() {
    let cap = make_test_capability();
    let output = format_capability(&cap, "json", true).unwrap();
    assert!(output.contains('\n'));
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["provider"], "test");
}

#[test]
fn format_yaml() {
    let cap = make_test_capability();
    let output = format_capability(&cap, "yaml", false).unwrap();
    assert!(output.contains("provider: test"));
}

#[test]
fn format_toml() {
    let cap = make_test_capability();
    let output = format_capability(&cap, "toml", false).unwrap();
    assert!(output.contains("provider = \"test\""));
}

#[test]
fn format_unsupported() {
    let cap = make_test_capability();
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
fn feature_support_constructors() {
    let native = FeatureSupport::native();
    assert!(native.supported);
    assert!(native.native);

    let wrapper = FeatureSupport::wrapper();
    assert!(wrapper.supported);
    assert!(!wrapper.native);

    let unsupported = FeatureSupport::unsupported();
    assert!(!unsupported.supported);
    assert!(!unsupported.native);
}

#[test]
fn session_log_support_constructors() {
    let full = SessionLogSupport::full();
    assert!(full.supported);
    assert_eq!(full.completeness, Some("full".to_string()));

    let partial = SessionLogSupport::partial();
    assert!(partial.supported);
    assert_eq!(partial.completeness, Some("partial".to_string()));

    let unsupported = SessionLogSupport::unsupported();
    assert!(!unsupported.supported);
    assert!(unsupported.completeness.is_none());
}

#[test]
fn session_logs_completeness_absent_when_unsupported() {
    let unsupported = SessionLogSupport::unsupported();
    let json = serde_json::to_string(&unsupported).unwrap();
    assert!(!json.contains("completeness"));
}

#[test]
fn models_to_vec_works() {
    let models = models_to_vec(&["a", "b", "c"]);
    assert_eq!(
        models,
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );
}

#[test]
fn capability_roundtrip() {
    let cap = make_test_capability();
    let json = serde_json::to_string(&cap).unwrap();
    let parsed: ProviderCapability = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.provider, "test");
    assert_eq!(parsed.available_models.len(), 2);
    assert!(parsed.features.interactive.supported);
}
