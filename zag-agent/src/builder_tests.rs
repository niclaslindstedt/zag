use super::*;

// ---------------------------------------------------------------------------
// Migrated existing tests
// ---------------------------------------------------------------------------

#[test]
fn test_builder_defaults() {
    let builder = AgentBuilder::new();
    assert!(builder.provider.is_none());
    assert!(builder.model.is_none());
    assert!(!builder.auto_approve);
    assert!(!builder.json_mode);
    assert!(!builder.verbose);
}

#[test]
fn test_builder_chaining() {
    let builder = AgentBuilder::new()
        .provider("claude")
        .model("sonnet")
        .auto_approve(true)
        .root("/tmp/test")
        .system_prompt("test prompt")
        .verbose(true)
        .json();

    assert_eq!(builder.provider.as_deref(), Some("claude"));
    assert_eq!(builder.model.as_deref(), Some("sonnet"));
    assert!(builder.auto_approve);
    assert_eq!(builder.root.as_deref(), Some("/tmp/test"));
    assert_eq!(builder.system_prompt.as_deref(), Some("test prompt"));
    assert!(builder.verbose);
    assert!(builder.json_mode);
}

#[test]
fn test_builder_json_schema_implies_json() {
    let schema = serde_json::json!({"type": "object"});
    let builder = AgentBuilder::new().json_schema(schema);
    assert!(builder.json_mode);
    assert!(builder.json_schema.is_some());
}

#[test]
fn test_builder_add_dirs() {
    let builder = AgentBuilder::new()
        .add_dir("/tmp/dir1")
        .add_dir("/tmp/dir2");
    assert_eq!(builder.add_dirs.len(), 2);
}

#[test]
fn test_builder_env_vars() {
    let builder = AgentBuilder::new().env("FOO", "bar").env("BAZ", "qux");
    assert_eq!(builder.env_vars.len(), 2);
    assert_eq!(builder.env_vars[0], ("FOO".to_string(), "bar".to_string()));
    assert_eq!(builder.env_vars[1], ("BAZ".to_string(), "qux".to_string()));
}

#[test]
fn test_resolve_provider_default() {
    let builder = AgentBuilder::new();
    // Default provider is "claude" (or whatever is in config)
    let provider = builder.resolve_provider().unwrap();
    assert!(!provider.is_empty());
}

#[test]
fn test_resolve_provider_explicit() {
    let builder = AgentBuilder::new().provider("gemini");
    assert_eq!(builder.resolve_provider().unwrap(), "gemini");
}

#[test]
fn test_resolve_provider_invalid() {
    let builder = AgentBuilder::new().provider("invalid");
    assert!(builder.resolve_provider().is_err());
}

#[test]
fn test_builder_streaming_flags() {
    let builder = AgentBuilder::new()
        .provider("claude")
        .replay_user_messages(true)
        .include_partial_messages(true);

    assert!(builder.replay_user_messages);
    assert!(builder.include_partial_messages);
}

#[test]
#[ignore] // requires 'claude' CLI installed in PATH
fn test_create_agent_claude() {
    let builder = AgentBuilder::new().provider("claude");
    let provider = builder.resolve_provider().unwrap();
    let agent = builder.create_agent(&provider).unwrap();
    assert_eq!(agent.name(), "claude");
}

#[test]
#[ignore] // requires 'claude' CLI installed in PATH
fn test_create_agent_with_model() {
    let builder = AgentBuilder::new().provider("claude").model("sonnet");
    let provider = builder.resolve_provider().unwrap();
    let agent = builder.create_agent(&provider).unwrap();
    assert_eq!(agent.get_model(), "sonnet");
}

// ---------------------------------------------------------------------------
// New setter tests
// ---------------------------------------------------------------------------

#[test]
fn test_builder_quiet_setter() {
    let builder = AgentBuilder::new().quiet(true);
    assert!(builder.quiet);

    let builder = AgentBuilder::new().quiet(false);
    assert!(!builder.quiet);
}

#[test]
fn test_builder_show_usage_setter() {
    let builder = AgentBuilder::new().show_usage(true);
    assert!(builder.show_usage);

    let builder = AgentBuilder::new().show_usage(false);
    assert!(!builder.show_usage);
}

#[test]
fn test_builder_max_turns_setter() {
    let builder = AgentBuilder::new().max_turns(5);
    assert_eq!(builder.max_turns, Some(5));
}

#[test]
fn test_builder_size_setter() {
    let builder = AgentBuilder::new().size("2b");
    assert_eq!(builder.size.as_deref(), Some("2b"));
}

#[test]
fn test_builder_session_id_setter() {
    let builder = AgentBuilder::new().session_id("abc-123");
    assert_eq!(builder.session_id.as_deref(), Some("abc-123"));
}

#[test]
fn test_builder_output_format_setter() {
    let builder = AgentBuilder::new().output_format("json");
    assert_eq!(builder.output_format.as_deref(), Some("json"));
}

#[test]
fn test_builder_input_format_setter() {
    let builder = AgentBuilder::new().input_format("stream-json");
    assert_eq!(builder.input_format.as_deref(), Some("stream-json"));
}

#[test]
fn test_builder_worktree_none() {
    let builder = AgentBuilder::new().worktree(None);
    assert_eq!(builder.worktree, Some(None));
}

#[test]
fn test_builder_worktree_some() {
    let builder = AgentBuilder::new().worktree(Some("my-wt"));
    assert_eq!(builder.worktree, Some(Some("my-wt".to_string())));
}

#[test]
fn test_builder_sandbox_none() {
    let builder = AgentBuilder::new().sandbox(None);
    assert_eq!(builder.sandbox, Some(None));
}

#[test]
fn test_builder_sandbox_some() {
    let builder = AgentBuilder::new().sandbox(Some("my-sb"));
    assert_eq!(builder.sandbox, Some(Some("my-sb".to_string())));
}

#[test]
fn test_builder_json_stream() {
    let builder = AgentBuilder::new().json_stream();
    assert!(builder.json_stream);
}

#[test]
fn test_resolve_provider_case_insensitive() {
    let builder = AgentBuilder::new().provider("CLAUDE");
    assert_eq!(builder.resolve_provider().unwrap(), "claude");

    let builder = AgentBuilder::new().provider("Gemini");
    assert_eq!(builder.resolve_provider().unwrap(), "gemini");

    let builder = AgentBuilder::new().provider("Codex");
    assert_eq!(builder.resolve_provider().unwrap(), "codex");
}

#[test]
fn test_builder_default_impl() {
    let from_default = AgentBuilder::default();
    let from_new = AgentBuilder::new();

    assert_eq!(from_default.provider, from_new.provider);
    assert_eq!(from_default.model, from_new.model);
    assert_eq!(from_default.auto_approve, from_new.auto_approve);
    assert_eq!(from_default.verbose, from_new.verbose);
    assert_eq!(from_default.quiet, from_new.quiet);
    assert_eq!(from_default.json_mode, from_new.json_mode);
    assert_eq!(from_default.json_stream, from_new.json_stream);
    assert_eq!(from_default.max_turns, from_new.max_turns);
    assert_eq!(from_default.show_usage, from_new.show_usage);
    assert_eq!(
        from_default.replay_user_messages,
        from_new.replay_user_messages
    );
    assert_eq!(
        from_default.include_partial_messages,
        from_new.include_partial_messages
    );
}

#[test]
fn test_resolve_provider_all_valid() {
    for provider in &["claude", "codex", "gemini", "copilot", "ollama", "mock"] {
        let builder = AgentBuilder::new().provider(provider);
        assert_eq!(builder.resolve_provider().unwrap(), *provider);
    }
}

// ---------------------------------------------------------------------------
// Mock agent builder integration
// ---------------------------------------------------------------------------

#[test]
fn test_resolve_provider_mock() {
    let builder = AgentBuilder::new().provider("mock");
    assert_eq!(builder.resolve_provider().unwrap(), "mock");
}

#[test]
fn test_resolve_provider_mock_case_insensitive() {
    let builder = AgentBuilder::new().provider("MOCK");
    assert_eq!(builder.resolve_provider().unwrap(), "mock");

    let builder = AgentBuilder::new().provider("Mock");
    assert_eq!(builder.resolve_provider().unwrap(), "mock");
}

#[test]
fn test_create_agent_mock() {
    let builder = AgentBuilder::new().provider("mock");
    let provider = builder.resolve_provider().unwrap();
    let agent = builder.create_agent(&provider).unwrap();
    assert_eq!(agent.name(), "mock");
    // Model depends on config (may be "mock-medium" if config has model = "medium")
    let model = agent.get_model();
    assert!(
        model.starts_with("mock-"),
        "Expected mock model, got: {}",
        model
    );
}

#[test]
fn test_create_agent_mock_with_model() {
    let builder = AgentBuilder::new().provider("mock").model("mock-large");
    let provider = builder.resolve_provider().unwrap();
    let agent = builder.create_agent(&provider).unwrap();
    assert_eq!(agent.get_model(), "mock-large");
}

#[test]
fn test_create_agent_mock_with_size_alias() {
    let builder = AgentBuilder::new().provider("mock").model("small");
    let provider = builder.resolve_provider().unwrap();
    let agent = builder.create_agent(&provider).unwrap();
    assert_eq!(agent.get_model(), "mock-small");
}

#[test]
fn test_create_agent_mock_with_auto_approve() {
    use crate::providers::mock::MockAgent;
    let builder = AgentBuilder::new().provider("mock").auto_approve(true);
    let provider = builder.resolve_provider().unwrap();
    let agent = builder.create_agent(&provider).unwrap();
    let mock = agent.as_any_ref().downcast_ref::<MockAgent>().unwrap();
    assert!(mock.skip_permissions());
}

#[test]
fn test_create_agent_mock_with_max_turns() {
    use crate::providers::mock::MockAgent;
    let builder = AgentBuilder::new().provider("mock").max_turns(10);
    let provider = builder.resolve_provider().unwrap();
    let agent = builder.create_agent(&provider).unwrap();
    let mock = agent.as_any_ref().downcast_ref::<MockAgent>().unwrap();
    assert_eq!(mock.max_turns(), Some(10));
}

#[test]
fn test_create_agent_mock_with_output_format() {
    use crate::providers::mock::MockAgent;
    let builder = AgentBuilder::new()
        .provider("mock")
        .output_format("stream-json");
    let provider = builder.resolve_provider().unwrap();
    let agent = builder.create_agent(&provider).unwrap();
    let mock = agent.as_any_ref().downcast_ref::<MockAgent>().unwrap();
    assert_eq!(mock.output_format(), Some("stream-json"));
}

#[test]
fn test_create_agent_mock_json_mode_sets_output_format() {
    use crate::providers::mock::MockAgent;
    let builder = AgentBuilder::new().provider("mock").json();
    let provider = builder.resolve_provider().unwrap();
    let agent = builder.create_agent(&provider).unwrap();
    let mock = agent.as_any_ref().downcast_ref::<MockAgent>().unwrap();
    assert_eq!(mock.output_format(), Some("json"));
}

#[test]
fn test_create_agent_mock_json_mode_augments_system_prompt() {
    let builder = AgentBuilder::new()
        .provider("mock")
        .system_prompt("original prompt")
        .json();
    let provider = builder.resolve_provider().unwrap();
    let agent = builder.create_agent(&provider).unwrap();
    assert!(agent.system_prompt().contains("original prompt"));
    assert!(agent.system_prompt().contains("valid JSON"));
}

#[test]
fn test_create_agent_mock_json_schema_augments_system_prompt() {
    let schema = serde_json::json!({"type": "object"});
    let builder = AgentBuilder::new().provider("mock").json_schema(schema);
    let provider = builder.resolve_provider().unwrap();
    let agent = builder.create_agent(&provider).unwrap();
    assert!(agent.system_prompt().contains("JSON schema"));
}
