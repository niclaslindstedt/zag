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
            streaming_input: StreamingInputSupport::unsupported(),
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

#[test]
fn session_log_partial_serialization() {
    let partial = SessionLogSupport::partial();
    let json = serde_json::to_string(&partial).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["supported"], true);
    assert_eq!(parsed["native"], true);
    assert_eq!(parsed["completeness"], "partial");
}

#[test]
fn session_log_full_serialization() {
    let full = SessionLogSupport::full();
    let json = serde_json::to_string(&full).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["supported"], true);
    assert_eq!(parsed["native"], true);
    assert_eq!(parsed["completeness"], "full");
}

#[test]
fn streaming_input_support_constructors() {
    let queue = StreamingInputSupport::queue();
    assert!(queue.supported);
    assert!(queue.native);
    assert_eq!(queue.semantics, Some("queue".to_string()));

    let interrupt = StreamingInputSupport::interrupt();
    assert!(interrupt.supported);
    assert!(interrupt.native);
    assert_eq!(interrupt.semantics, Some("interrupt".to_string()));

    let between = StreamingInputSupport::between_turns_only();
    assert!(between.supported);
    assert!(between.native);
    assert_eq!(between.semantics, Some("between-turns-only".to_string()));

    let unsupported = StreamingInputSupport::unsupported();
    assert!(!unsupported.supported);
    assert!(!unsupported.native);
    assert!(unsupported.semantics.is_none());
}

#[test]
fn streaming_input_semantics_absent_when_unsupported() {
    let unsupported = StreamingInputSupport::unsupported();
    let json = serde_json::to_string(&unsupported).unwrap();
    assert!(!json.contains("semantics"));
}

#[test]
fn streaming_input_queue_serialization() {
    let queue = StreamingInputSupport::queue();
    let json = serde_json::to_string(&queue).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["supported"], true);
    assert_eq!(parsed["native"], true);
    assert_eq!(parsed["semantics"], "queue");
}

#[test]
fn streaming_input_unsupported_deserialize_roundtrip() {
    let unsupported = StreamingInputSupport::unsupported();
    let json = serde_json::to_string(&unsupported).unwrap();
    let parsed: StreamingInputSupport = serde_json::from_str(&json).unwrap();
    assert!(!parsed.supported);
    assert!(!parsed.native);
    assert!(parsed.semantics.is_none());
}

#[test]
fn claude_streaming_input_is_queue() {
    let cap = get_capability("claude").unwrap();
    assert!(cap.features.streaming_input.supported);
    assert!(cap.features.streaming_input.native);
    assert_eq!(
        cap.features.streaming_input.semantics,
        Some("queue".to_string())
    );
}

#[test]
fn non_claude_providers_have_no_streaming_input_semantics() {
    for provider in ["codex", "gemini", "copilot", "ollama"] {
        let cap = get_capability(provider).unwrap();
        assert!(!cap.features.streaming_input.supported);
        assert!(cap.features.streaming_input.semantics.is_none());
    }
}

#[test]
fn session_log_unsupported_deserialize_roundtrip() {
    let unsupported = SessionLogSupport::unsupported();
    let json = serde_json::to_string(&unsupported).unwrap();
    let parsed: SessionLogSupport = serde_json::from_str(&json).unwrap();
    assert!(!parsed.supported);
    assert!(!parsed.native);
    assert!(parsed.completeness.is_none());
}

#[test]
fn feature_support_serialization_roundtrip() {
    for support in [
        FeatureSupport::native(),
        FeatureSupport::wrapper(),
        FeatureSupport::unsupported(),
    ] {
        let json = serde_json::to_string(&support).unwrap();
        let parsed: FeatureSupport = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.supported, support.supported);
        assert_eq!(parsed.native, support.native);
    }
}

#[test]
fn list_providers_returns_all_real_providers() {
    let providers = list_providers();
    assert_eq!(providers.len(), 5);
    assert!(providers.contains(&"claude".to_string()));
    assert!(providers.contains(&"codex".to_string()));
    assert!(providers.contains(&"gemini".to_string()));
    assert!(providers.contains(&"copilot".to_string()));
    assert!(providers.contains(&"ollama".to_string()));
    assert!(!providers.contains(&"auto".to_string()));
    assert!(!providers.contains(&"mock".to_string()));
}

#[test]
fn get_all_capabilities_returns_all_providers() {
    let caps = get_all_capabilities();
    assert_eq!(caps.len(), 5);
    let names: Vec<&str> = caps.iter().map(|c| c.provider.as_str()).collect();
    assert!(names.contains(&"claude"));
    assert!(names.contains(&"codex"));
    assert!(names.contains(&"gemini"));
    assert!(names.contains(&"copilot"));
    assert!(names.contains(&"ollama"));
    for cap in &caps {
        assert!(!cap.available_models.is_empty());
        assert!(!cap.default_model.is_empty());
    }
}

#[test]
fn resolve_model_alias() {
    let rm = resolve_model("claude", "default").unwrap();
    assert_eq!(rm.resolved, "sonnet");
    assert!(rm.is_alias);
    assert_eq!(rm.provider, "claude");
}

#[test]
fn resolve_model_size_alias() {
    let rm = resolve_model("codex", "small").unwrap();
    assert_eq!(rm.resolved, "gpt-5.4-mini");
    assert!(rm.is_alias);
}

#[test]
fn resolve_model_passthrough() {
    let rm = resolve_model("claude", "opus").unwrap();
    assert_eq!(rm.resolved, "opus");
    assert!(!rm.is_alias);
}

#[test]
fn resolve_model_unknown_provider() {
    let result = resolve_model("nonexistent", "model");
    assert!(result.is_err());
}

#[test]
fn resolved_model_serialization() {
    let rm = resolve_model("claude", "small").unwrap();
    let json = serde_json::to_string(&rm).unwrap();
    let parsed: ResolvedModel = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.input, "small");
    assert_eq!(parsed.resolved, "haiku");
    assert!(parsed.is_alias);
    assert_eq!(parsed.provider, "claude");
}
