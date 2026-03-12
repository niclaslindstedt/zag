use super::*;

// --- create_agent ---

#[test]
fn test_create_agent_claude() {
    let agent = AgentFactory::create_agent("claude").unwrap();
    assert_eq!(agent.name(), "claude");
}

#[test]
fn test_create_agent_codex() {
    let agent = AgentFactory::create_agent("codex").unwrap();
    assert_eq!(agent.name(), "codex");
}

#[test]
fn test_create_agent_gemini() {
    let agent = AgentFactory::create_agent("gemini").unwrap();
    assert_eq!(agent.name(), "gemini");
}

#[test]
fn test_create_agent_copilot() {
    let agent = AgentFactory::create_agent("copilot").unwrap();
    assert_eq!(agent.name(), "copilot");
}

#[test]
fn test_create_agent_case_insensitive() {
    let agent = AgentFactory::create_agent("Claude").unwrap();
    assert_eq!(agent.name(), "claude");
}

#[test]
fn test_create_agent_unknown() {
    let result = AgentFactory::create_agent("unknown");
    let err = result.err().expect("Expected an error");
    assert!(err.to_string().contains("Unknown agent"));
}

// --- resolve_model ---

#[test]
fn test_resolve_model_size_alias() {
    assert_eq!(AgentFactory::resolve_model("claude", "small"), "haiku");
    assert_eq!(AgentFactory::resolve_model("codex", "large"), "gpt-5.1-codex-max");
    assert_eq!(AgentFactory::resolve_model("gemini", "medium"), "gemini-2.5-flash");
    assert_eq!(AgentFactory::resolve_model("copilot", "small"), "claude-haiku-4.5");
}

#[test]
fn test_resolve_model_passthrough() {
    assert_eq!(AgentFactory::resolve_model("claude", "opus"), "opus");
    assert_eq!(AgentFactory::resolve_model("codex", "gpt-5.2"), "gpt-5.2");
}

#[test]
fn test_resolve_model_unknown_agent_passthrough() {
    assert_eq!(AgentFactory::resolve_model("unknown", "whatever"), "whatever");
}

// --- validate_model ---

#[test]
fn test_validate_model_valid() {
    assert!(AgentFactory::validate_model("claude", "sonnet").is_ok());
    assert!(AgentFactory::validate_model("codex", "gpt-5.2-codex").is_ok());
    assert!(AgentFactory::validate_model("gemini", "auto").is_ok());
    assert!(AgentFactory::validate_model("copilot", "claude-sonnet-4.5").is_ok());
}

#[test]
fn test_validate_model_invalid() {
    let result = AgentFactory::validate_model("claude", "gpt-5");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid model"));
}

#[test]
fn test_validate_model_unknown_agent_skips() {
    assert!(AgentFactory::validate_model("unknown", "anything").is_ok());
}

// --- create (integration) ---

#[test]
fn test_create_with_model_resolution() {
    let agent = AgentFactory::create("claude", None, Some("small".to_string()), None, false, vec![]).unwrap();
    assert_eq!(agent.get_model(), "haiku");
}

#[test]
fn test_create_with_specific_model() {
    let agent = AgentFactory::create("claude", None, Some("sonnet".to_string()), None, false, vec![]).unwrap();
    assert_eq!(agent.get_model(), "sonnet");
}

#[test]
fn test_create_with_invalid_model() {
    let result = AgentFactory::create("claude", None, Some("gpt-5".to_string()), None, false, vec![]);
    assert!(result.is_err());
}

#[test]
fn test_create_with_system_prompt() {
    let agent = AgentFactory::create("claude", Some("test prompt".to_string()), None, None, false, vec![]).unwrap();
    assert_eq!(agent.system_prompt(), "test prompt");
}

#[test]
fn test_create_default_uses_config_or_agent_default() {
    // When no model is specified, the factory uses config > agent default
    // The actual model depends on the config file in the current repo
    let agent = AgentFactory::create("claude", None, None, None, false, vec![]).unwrap();
    let model = agent.get_model();
    // Should be a valid claude model (either from config or default)
    assert!(
        ["sonnet", "opus", "haiku"].contains(&model),
        "unexpected model: {}",
        model
    );
}

#[test]
fn test_create_all_agents_default() {
    for name in &["claude", "codex", "gemini", "copilot"] {
        let agent = AgentFactory::create(name, None, None, None, false, vec![]).unwrap();
        assert_eq!(agent.name(), *name);
    }
}
