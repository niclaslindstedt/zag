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
    assert_eq!(AgentFactory::resolve_model("codex", "large"), "gpt-5.4");
    assert_eq!(
        AgentFactory::resolve_model("gemini", "medium"),
        "gemini-2.5-flash"
    );
    assert_eq!(
        AgentFactory::resolve_model("copilot", "small"),
        "claude-haiku-4.5"
    );
}

#[test]
fn test_resolve_model_passthrough() {
    assert_eq!(AgentFactory::resolve_model("claude", "opus"), "opus");
    assert_eq!(AgentFactory::resolve_model("codex", "gpt-5.2"), "gpt-5.2");
}

#[test]
fn test_resolve_model_unknown_agent_passthrough() {
    assert_eq!(
        AgentFactory::resolve_model("unknown", "whatever"),
        "whatever"
    );
}

// --- validate_model ---

#[test]
fn test_validate_model_valid() {
    assert!(AgentFactory::validate_model("claude", "sonnet").is_ok());
    assert!(AgentFactory::validate_model("codex", "gpt-5.4").is_ok());
    assert!(AgentFactory::validate_model("gemini", "auto").is_ok());
    assert!(AgentFactory::validate_model("copilot", "claude-sonnet-4.6").is_ok());
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
    if crate::preflight::check_binary("claude").is_err() {
        return; // Skip if claude CLI not available
    }
    let agent = AgentFactory::create(
        "claude",
        None,
        Some("small".to_string()),
        None,
        false,
        vec![],
    )
    .unwrap();
    assert_eq!(agent.get_model(), "haiku");
}

#[test]
fn test_create_with_specific_model() {
    if crate::preflight::check_binary("claude").is_err() {
        return;
    }
    let agent = AgentFactory::create(
        "claude",
        None,
        Some("sonnet".to_string()),
        None,
        false,
        vec![],
    )
    .unwrap();
    assert_eq!(agent.get_model(), "sonnet");
}

#[test]
fn test_create_with_invalid_model() {
    if crate::preflight::check_binary("claude").is_err() {
        return;
    }
    let result = AgentFactory::create(
        "claude",
        None,
        Some("gpt-5".to_string()),
        None,
        false,
        vec![],
    );
    assert!(result.is_err());
}

#[test]
fn test_create_with_system_prompt() {
    if crate::preflight::check_binary("claude").is_err() {
        return;
    }
    let agent = AgentFactory::create(
        "claude",
        Some("test prompt".to_string()),
        None,
        None,
        false,
        vec![],
    )
    .unwrap();
    assert_eq!(agent.system_prompt(), "test prompt");
}

#[test]
fn test_create_default_uses_config_or_agent_default() {
    if crate::preflight::check_binary("claude").is_err() {
        return;
    }
    // When no model is specified, the factory uses config > agent default
    // The actual model depends on the config file in the current repo
    let agent = AgentFactory::create("claude", None, None, None, false, vec![]).unwrap();
    let model = agent.get_model();
    // Should be a valid claude model (either from config or default)
    assert!(
        ["sonnet", "opus", "haiku"].contains(&model),
        "unexpected model: {model}"
    );
}

#[test]
fn test_create_missing_binary_gives_actionable_error() {
    let result = AgentFactory::create("zag-nonexistent-agent-xyz", None, None, None, false, vec![]);
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(err.contains("not found in PATH"));
}

// --- mock agent ---

#[test]
fn test_create_agent_mock() {
    let agent = AgentFactory::create_agent("mock").unwrap();
    assert_eq!(agent.name(), "mock");
}

#[test]
fn test_resolve_model_mock() {
    assert_eq!(AgentFactory::resolve_model("mock", "small"), "mock-small");
    assert_eq!(AgentFactory::resolve_model("mock", "medium"), "mock-medium");
    assert_eq!(AgentFactory::resolve_model("mock", "large"), "mock-large");
    assert_eq!(
        AgentFactory::resolve_model("mock", "mock-default"),
        "mock-default"
    );
}

#[test]
fn test_validate_model_mock() {
    assert!(AgentFactory::validate_model("mock", "mock-default").is_ok());
    assert!(AgentFactory::validate_model("mock", "mock-small").is_ok());
    assert!(AgentFactory::validate_model("mock", "mock-medium").is_ok());
    assert!(AgentFactory::validate_model("mock", "mock-large").is_ok());
    assert!(AgentFactory::validate_model("mock", "invalid").is_err());
}

#[test]
fn test_create_mock_no_preflight_needed() {
    // Mock agent skips preflight binary check
    let agent = AgentFactory::create("mock", None, None, None, false, vec![]).unwrap();
    assert_eq!(agent.name(), "mock");
}

#[test]
fn test_create_mock_with_model_resolution() {
    let agent =
        AgentFactory::create("mock", None, Some("small".to_string()), None, false, vec![]).unwrap();
    assert_eq!(agent.get_model(), "mock-small");
}

#[test]
fn test_create_mock_with_invalid_model() {
    let result = AgentFactory::create(
        "mock",
        None,
        Some("invalid".to_string()),
        None,
        false,
        vec![],
    );
    assert!(result.is_err());
}

#[test]
fn test_create_all_agents_default() {
    // Only test agents whose CLI binary is available in PATH.
    // The preflight check in create() validates binary availability.
    for name in &["claude", "codex", "gemini", "copilot"] {
        if crate::preflight::check_binary(name).is_ok() {
            let agent = AgentFactory::create(name, None, None, None, false, vec![]).unwrap();
            assert_eq!(agent.name(), *name);
        }
    }
}

// --- fallback_sequence ---

#[test]
fn test_fallback_sequence_starts_with_requested_provider() {
    let seq = fallback_sequence("gemini");
    assert_eq!(seq[0], "gemini");
}

#[test]
fn test_fallback_sequence_contains_every_tier_provider_once() {
    let seq = fallback_sequence("gemini");
    for p in PROVIDER_TIER_LIST {
        assert!(seq.contains(&p.to_string()), "missing: {p}");
    }
    // No duplicates.
    let mut sorted = seq.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), seq.len());
}

#[test]
fn test_fallback_sequence_unknown_start_is_prepended() {
    // A provider not in the tier list (e.g. "mock" from tests) is still
    // tried first — the tier list is a fallback, not a whitelist.
    let seq = fallback_sequence("mock");
    assert_eq!(seq[0], "mock");
    assert_eq!(seq.len(), PROVIDER_TIER_LIST.len() + 1);
}

#[test]
fn test_fallback_sequence_is_case_insensitive_start() {
    let seq = fallback_sequence("CLAUDE");
    assert_eq!(seq[0], "claude");
    // No duplicate "claude" sneaking in from the tier list.
    let claude_count = seq.iter().filter(|p| p.as_str() == "claude").count();
    assert_eq!(claude_count, 1);
}

// --- create_with_fallback ---

#[tokio::test]
async fn test_create_with_fallback_explicit_missing_binary_errors() {
    // Explicit pinning must not fall back — missing binary is a hard error.
    let mut calls: Vec<(String, String, String)> = Vec::new();
    let mut on_downgrade = |from: &str, to: &str, reason: &str| {
        calls.push((from.to_string(), to.to_string(), reason.to_string()));
    };
    let result = AgentFactory::create_with_fallback(
        "zag-nonexistent-agent-xyz",
        true,
        None,
        None,
        None,
        false,
        vec![],
        &mut on_downgrade,
    )
    .await;
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(err.contains("not found in PATH"), "got: {err}");
    assert!(calls.is_empty(), "explicit pinning must not downgrade");
}

#[tokio::test]
async fn test_create_with_fallback_non_explicit_downgrades_to_mock() {
    // Non-explicit: first candidate is bogus, expect a downgrade notification
    // and ultimately either a working real provider OR a final error.
    let mut calls: Vec<(String, String, String)> = Vec::new();
    let mut on_downgrade = |from: &str, to: &str, reason: &str| {
        calls.push((from.to_string(), to.to_string(), reason.to_string()));
    };
    let result = AgentFactory::create_with_fallback(
        "zag-nonexistent-agent-xyz",
        false,
        None,
        None,
        None,
        false,
        vec![],
        &mut on_downgrade,
    )
    .await;

    // At least one downgrade must have been signalled, because the first
    // candidate is guaranteed to fail the preflight check.
    assert!(
        !calls.is_empty(),
        "expected at least one downgrade call, got none"
    );
    assert_eq!(calls[0].0, "zag-nonexistent-agent-xyz");
    assert!(calls[0].2.contains("not found in PATH"));

    // Whether the final `result` is Ok depends on which of the real
    // provider binaries happen to be available in the test PATH. The
    // callback firing is what we actually care about for this test.
    let _ = result;
}

#[tokio::test]
async fn test_create_with_fallback_non_explicit_all_missing_errors() {
    // Shadow PATH so none of the real provider binaries are findable.
    // Safety: single-threaded tokio test with set_var / remove_var.
    // (Rust 1.82+ marks these unsafe in multi-threaded contexts.)
    let original = std::env::var_os("PATH");
    // SAFETY: tests run in a single-threaded tokio runtime, no concurrent
    // access to environment variables.
    unsafe {
        std::env::set_var("PATH", "/nonexistent-zag-test-path");
    }

    let mut calls: Vec<(String, String, String)> = Vec::new();
    let mut on_downgrade = |from: &str, to: &str, reason: &str| {
        calls.push((from.to_string(), to.to_string(), reason.to_string()));
    };
    let result = AgentFactory::create_with_fallback(
        "claude",
        false,
        None,
        None,
        None,
        false,
        vec![],
        &mut on_downgrade,
    )
    .await;

    // Restore PATH before assertions so a failing test doesn't leak.
    // SAFETY: same as above.
    unsafe {
        match original {
            Some(p) => std::env::set_var("PATH", p),
            None => std::env::remove_var("PATH"),
        }
    }

    assert!(
        result.is_err(),
        "expected final error with no working provider"
    );
    // Every tier entry except the last should have triggered a downgrade.
    assert!(
        calls.len() >= PROVIDER_TIER_LIST.len() - 1,
        "expected downgrade notifications, got {} calls",
        calls.len()
    );
}
