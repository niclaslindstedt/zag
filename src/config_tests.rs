use super::*;

#[test]
fn test_default_config() {
    let config = Config::default();
    assert!(config.defaults.auto_approve.is_none());
    assert!(config.models.claude.is_none());
}

#[test]
fn test_parse_config() {
    let toml = r#"
[defaults]
auto_approve = true

[models]
claude = "sonnet"
codex = "gpt-5.1-codex-mini"
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.defaults.auto_approve, Some(true));
    assert_eq!(config.models.claude, Some("sonnet".to_string()));
    assert_eq!(config.models.codex, Some("gpt-5.1-codex-mini".to_string()));
    assert!(config.models.gemini.is_none());
}

#[test]
fn test_get_model() {
    let config = Config {
        models: AgentModels {
            claude: Some("opus".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(config.get_model("claude"), Some("opus"));
    assert_eq!(config.get_model("codex"), None);
}

#[test]
fn test_get_model_falls_back_to_default() {
    let config = Config {
        defaults: Defaults {
            model: Some("large".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    // No agent-specific model, so falls back to defaults.model
    assert_eq!(config.get_model("claude"), Some("large"));
    assert_eq!(config.get_model("codex"), Some("large"));
}

#[test]
fn test_get_model_agent_specific_overrides_default() {
    let config = Config {
        defaults: Defaults {
            model: Some("small".to_string()),
            ..Default::default()
        },
        models: AgentModels {
            claude: Some("opus".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(config.get_model("claude"), Some("opus"));
    assert_eq!(config.get_model("codex"), Some("small"));
}

#[test]
fn test_get_model_unknown_agent() {
    let config = Config {
        defaults: Defaults {
            model: Some("medium".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    // Unknown agent falls back to default
    assert_eq!(config.get_model("unknown"), Some("medium"));
}

#[test]
fn test_provider_config() {
    let toml = r#"
[defaults]
provider = "gemini"
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.provider(), Some("gemini"));
}

#[test]
fn test_auto_approve() {
    let config = Config::default();
    assert!(!config.auto_approve());

    let config = Config {
        defaults: Defaults {
            auto_approve: Some(true),
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(config.auto_approve());

    let config = Config {
        defaults: Defaults {
            auto_approve: Some(false),
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(!config.auto_approve());
}

#[test]
fn test_default_model() {
    let config = Config::default();
    assert_eq!(config.default_model(), None);

    let config = Config {
        defaults: Defaults {
            model: Some("large".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(config.default_model(), Some("large"));
}

#[test]
fn test_get_value() {
    let config = Config {
        defaults: Defaults {
            provider: Some("codex".to_string()),
            model: Some("large".to_string()),
            auto_approve: Some(true),
        },
        models: AgentModels {
            claude: Some("opus".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(config.get_value("provider"), Some("codex".to_string()));
    assert_eq!(config.get_value("model"), Some("large".to_string()));
    assert_eq!(config.get_value("auto_approve"), Some("true".to_string()));
    assert_eq!(config.get_value("model.claude"), Some("opus".to_string()));
    assert_eq!(config.get_value("model.codex"), None);
    assert_eq!(config.get_value("model.gemini"), None);
    assert_eq!(config.get_value("model.copilot"), None);
    assert_eq!(config.get_value("unknown"), None);
}

#[test]
fn test_set_value() {
    let mut config = Config::default();

    config.set_value("provider", "gemini").unwrap();
    assert_eq!(config.defaults.provider, Some("gemini".to_string()));

    config.set_value("model", "large").unwrap();
    assert_eq!(config.defaults.model, Some("large".to_string()));

    config.set_value("auto_approve", "true").unwrap();
    assert_eq!(config.defaults.auto_approve, Some(true));

    config.set_value("model.claude", "opus").unwrap();
    assert_eq!(config.models.claude, Some("opus".to_string()));

    config.set_value("model.codex", "gpt-5.2").unwrap();
    assert_eq!(config.models.codex, Some("gpt-5.2".to_string()));

    config.set_value("model.gemini", "auto").unwrap();
    assert_eq!(config.models.gemini, Some("auto".to_string()));

    config.set_value("model.copilot", "gpt-5").unwrap();
    assert_eq!(config.models.copilot, Some("gpt-5".to_string()));
}

#[test]
fn test_set_value_auto_approve_variants() {
    let mut config = Config::default();

    for truthy in &["true", "1", "yes", "TRUE", "Yes"] {
        config.set_value("auto_approve", truthy).unwrap();
        assert_eq!(config.defaults.auto_approve, Some(true));
    }

    for falsy in &["false", "0", "no", "FALSE", "No"] {
        config.set_value("auto_approve", falsy).unwrap();
        assert_eq!(config.defaults.auto_approve, Some(false));
    }
}

#[test]
fn test_set_value_provider_case_insensitive() {
    let mut config = Config::default();
    config.set_value("provider", "CLAUDE").unwrap();
    assert_eq!(config.defaults.provider, Some("claude".to_string()));

    config.set_value("provider", "Gemini").unwrap();
    assert_eq!(config.defaults.provider, Some("gemini".to_string()));
}

#[test]
fn test_set_value_invalid_provider() {
    let mut config = Config::default();
    let result = config.set_value("provider", "invalid");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid provider"));
}

#[test]
fn test_set_value_invalid_auto_approve() {
    let mut config = Config::default();
    let result = config.set_value("auto_approve", "maybe");
    assert!(result.is_err());
}

#[test]
fn test_set_value_unknown_key() {
    let mut config = Config::default();
    let result = config.set_value("unknown_key", "value");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Unknown config key")
    );
}

#[test]
fn test_valid_providers() {
    assert!(Config::VALID_PROVIDERS.contains(&"claude"));
    assert!(Config::VALID_PROVIDERS.contains(&"codex"));
    assert!(Config::VALID_PROVIDERS.contains(&"gemini"));
    assert!(Config::VALID_PROVIDERS.contains(&"copilot"));
    assert!(!Config::VALID_PROVIDERS.contains(&"openai"));
}

#[test]
fn test_parse_empty_config() {
    let config: Config = toml::from_str("").unwrap();
    assert!(config.defaults.provider.is_none());
    assert!(config.defaults.model.is_none());
    assert!(config.defaults.auto_approve.is_none());
}

#[test]
fn test_parse_full_config() {
    let toml = r#"
[defaults]
provider = "codex"
model = "large"
auto_approve = false

[models]
claude = "opus"
codex = "gpt-5.2-codex"
gemini = "auto"
copilot = "claude-sonnet-4.5"
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.defaults.provider, Some("codex".to_string()));
    assert_eq!(config.defaults.model, Some("large".to_string()));
    assert_eq!(config.defaults.auto_approve, Some(false));
    assert_eq!(config.models.claude, Some("opus".to_string()));
    assert_eq!(config.models.codex, Some("gpt-5.2-codex".to_string()));
    assert_eq!(config.models.gemini, Some("auto".to_string()));
    assert_eq!(config.models.copilot, Some("claude-sonnet-4.5".to_string()));
}

#[test]
fn test_config_serialization_roundtrip() {
    let config = Config {
        defaults: Defaults {
            provider: Some("claude".to_string()),
            model: Some("medium".to_string()),
            auto_approve: Some(true),
        },
        models: AgentModels {
            claude: Some("opus".to_string()),
            codex: None,
            gemini: None,
            copilot: None,
        },
        ..Default::default()
    };
    let serialized = toml::to_string_pretty(&config).unwrap();
    let deserialized: Config = toml::from_str(&serialized).unwrap();
    assert_eq!(deserialized.defaults.provider, Some("claude".to_string()));
    assert_eq!(deserialized.models.claude, Some("opus".to_string()));
}

#[test]
fn test_config_path_with_root() {
    let path = Config::config_path(Some("/tmp/test"));
    assert_eq!(
        path,
        std::path::PathBuf::from("/tmp/test/.agent/agent.toml")
    );
}

#[test]
fn test_agent_dir_with_root() {
    let dir = Config::agent_dir(Some("/tmp/test"));
    assert_eq!(dir, std::path::PathBuf::from("/tmp/test/.agent"));
}
