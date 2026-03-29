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
codex = "gpt-5.4-mini"
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.defaults.auto_approve, Some(true));
    assert_eq!(config.models.claude, Some("sonnet".to_string()));
    assert_eq!(config.models.codex, Some("gpt-5.4-mini".to_string()));
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
            ..Default::default()
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

    config.set_value("model.codex", "gpt-5.4").unwrap();
    assert_eq!(config.models.codex, Some("gpt-5.4".to_string()));

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
codex = "gpt-5.4"
gemini = "auto"
copilot = "claude-sonnet-4.5"
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.defaults.provider, Some("codex".to_string()));
    assert_eq!(config.defaults.model, Some("large".to_string()));
    assert_eq!(config.defaults.auto_approve, Some(false));
    assert_eq!(config.models.claude, Some("opus".to_string()));
    assert_eq!(config.models.codex, Some("gpt-5.4".to_string()));
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
            max_turns: Some(10),
            system_prompt: Some("Be helpful".to_string()),
        },
        models: AgentModels {
            claude: Some("opus".to_string()),
            codex: None,
            gemini: None,
            copilot: None,
            ollama: None,
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
    let home = dirs::home_dir().unwrap();
    assert_eq!(path, home.join(".zag/projects/tmp-test/zag.toml"));
}

#[test]
fn test_sanitize_path() {
    assert_eq!(
        Config::sanitize_path("/Users/foo/Source/agent"),
        "Users-foo-Source-agent"
    );
    assert_eq!(
        Config::sanitize_path("/home/user/projects/my-app"),
        "home-user-projects-my-app"
    );
    assert_eq!(Config::sanitize_path("relative/path"), "relative-path");
}

// --- Auto config ---

#[test]
fn test_auto_provider_getter() {
    let config = Config {
        auto: AutoConfig {
            provider: Some("gemini".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(config.auto_provider(), Some("gemini"));
}

#[test]
fn test_auto_model_getter() {
    let config = Config {
        auto: AutoConfig {
            model: Some("haiku".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(config.auto_model(), Some("haiku"));
}

#[test]
fn test_auto_config_defaults_none() {
    let config = Config::default();
    assert_eq!(config.auto_provider(), None);
    assert_eq!(config.auto_model(), None);
}

#[test]
fn test_set_value_auto_provider() {
    let mut config = Config::default();
    config.set_value("auto.provider", "codex").unwrap();
    assert_eq!(config.auto.provider, Some("codex".to_string()));
}

#[test]
fn test_set_value_auto_model() {
    let mut config = Config::default();
    config.set_value("auto.model", "haiku").unwrap();
    assert_eq!(config.auto.model, Some("haiku".to_string()));
}

#[test]
fn test_get_value_auto_fields() {
    let config = Config {
        auto: AutoConfig {
            provider: Some("claude".to_string()),
            model: Some("sonnet".to_string()),
        },
        ..Default::default()
    };
    assert_eq!(
        config.get_value("auto.provider"),
        Some("claude".to_string())
    );
    assert_eq!(config.get_value("auto.model"), Some("sonnet".to_string()));
}

#[test]
fn test_parse_auto_config() {
    let toml = r#"
[auto]
provider = "gemini"
model = "haiku"
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.auto_provider(), Some("gemini"));
    assert_eq!(config.auto_model(), Some("haiku"));
}

// --- Init and file I/O ---

fn temp_root(suffix: &str) -> (String, impl Drop) {
    let dir =
        std::env::temp_dir().join(format!("zag-config-test-{}-{}", std::process::id(), suffix));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let root = dir.to_str().unwrap().to_string();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    (root, Cleanup(dir))
}

#[test]
fn test_save_and_load_roundtrip() {
    let (root, _guard) = temp_root("roundtrip");
    let mut config = Config::default();
    config.defaults.provider = Some("codex".to_string());
    config.defaults.model = Some("large".to_string());
    config.models.claude = Some("opus".to_string());
    config.auto.provider = Some("gemini".to_string());
    config.save(Some(&root)).unwrap();

    let loaded = Config::load(Some(&root)).unwrap();
    assert_eq!(loaded.defaults.provider, Some("codex".to_string()));
    assert_eq!(loaded.defaults.model, Some("large".to_string()));
    assert_eq!(loaded.models.claude, Some("opus".to_string()));
    assert_eq!(loaded.auto.provider, Some("gemini".to_string()));
}

#[test]
fn test_load_missing_file_returns_default() {
    let (root, _guard) = temp_root("missing");
    let config = Config::load(Some(&root)).unwrap();
    assert!(config.defaults.provider.is_none());
}

#[test]
fn test_init_creates_config() {
    let (root, _guard) = temp_root("init");
    let created = Config::init(Some(&root)).unwrap();
    assert!(created);
    assert!(Config::config_path(Some(&root)).exists());

    // Calling init again should return false (already exists)
    let created_again = Config::init(Some(&root)).unwrap();
    assert!(!created_again);
}

#[test]
fn test_global_logs_dir_not_empty() {
    let dir = Config::global_logs_dir();
    assert!(dir.to_str().unwrap().contains("logs"));
}

#[test]
fn test_agent_dir_with_root() {
    let dir = Config::agent_dir(Some("/tmp/test"));
    let home = dirs::home_dir().unwrap();
    assert_eq!(dir, home.join(".zag/projects/tmp-test"));
}

// --- max_turns config ---

#[test]
fn test_max_turns_getter() {
    let config = Config::default();
    assert_eq!(config.max_turns(), None);

    let config = Config {
        defaults: Defaults {
            max_turns: Some(10),
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(config.max_turns(), Some(10));
}

#[test]
fn test_set_value_max_turns() {
    let mut config = Config::default();
    config.set_value("max_turns", "5").unwrap();
    assert_eq!(config.defaults.max_turns, Some(5));
}

#[test]
fn test_set_value_max_turns_invalid() {
    let mut config = Config::default();
    let result = config.set_value("max_turns", "abc");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("positive integer"));
}

#[test]
fn test_get_value_max_turns() {
    let config = Config {
        defaults: Defaults {
            max_turns: Some(15),
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(config.get_value("max_turns"), Some("15".to_string()));
}

#[test]
fn test_parse_max_turns_config() {
    let toml = r#"
[defaults]
max_turns = 20
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.max_turns(), Some(20));
}

// --- system_prompt config ---

#[test]
fn test_system_prompt_getter() {
    let config = Config::default();
    assert_eq!(config.system_prompt(), None);

    let config = Config {
        defaults: Defaults {
            system_prompt: Some("You are a Rust expert".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(config.system_prompt(), Some("You are a Rust expert"));
}

#[test]
fn test_set_value_system_prompt() {
    let mut config = Config::default();
    config.set_value("system_prompt", "Be concise").unwrap();
    assert_eq!(
        config.defaults.system_prompt,
        Some("Be concise".to_string())
    );
}

#[test]
fn test_get_value_system_prompt() {
    let config = Config {
        defaults: Defaults {
            system_prompt: Some("Test prompt".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        config.get_value("system_prompt"),
        Some("Test prompt".to_string())
    );
}

#[test]
fn test_parse_system_prompt_config() {
    let toml = r#"
[defaults]
system_prompt = "You are helpful"
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.system_prompt(), Some("You are helpful"));
}

// --- unset_value ---

#[test]
fn test_unset_value() {
    let mut config = Config {
        defaults: Defaults {
            provider: Some("codex".to_string()),
            model: Some("large".to_string()),
            auto_approve: Some(true),
            max_turns: Some(10),
            system_prompt: Some("test".to_string()),
        },
        models: AgentModels {
            claude: Some("opus".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    config.unset_value("provider").unwrap();
    assert!(config.defaults.provider.is_none());

    config.unset_value("model").unwrap();
    assert!(config.defaults.model.is_none());

    config.unset_value("auto_approve").unwrap();
    assert!(config.defaults.auto_approve.is_none());

    config.unset_value("max_turns").unwrap();
    assert!(config.defaults.max_turns.is_none());

    config.unset_value("system_prompt").unwrap();
    assert!(config.defaults.system_prompt.is_none());

    config.unset_value("model.claude").unwrap();
    assert!(config.models.claude.is_none());
}

#[test]
fn test_unset_value_unknown_key() {
    let mut config = Config::default();
    let result = config.unset_value("nonexistent");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Unknown config key")
    );
}

// --- Ollama config ---

#[test]
fn test_set_and_get_ollama_config() {
    let mut config = Config::default();
    config.set_value("ollama.model", "llama3").unwrap();
    assert_eq!(config.get_value("ollama.model"), Some("llama3".to_string()));
    config.set_value("ollama.size", "70b").unwrap();
    assert_eq!(config.get_value("ollama.size"), Some("70b".to_string()));
    config.set_value("ollama.size_small", "1b").unwrap();
    assert_eq!(
        config.get_value("ollama.size_small"),
        Some("1b".to_string())
    );
    config.set_value("ollama.size_medium", "14b").unwrap();
    assert_eq!(
        config.get_value("ollama.size_medium"),
        Some("14b".to_string())
    );
    config.set_value("ollama.size_large", "70b").unwrap();
    assert_eq!(
        config.get_value("ollama.size_large"),
        Some("70b".to_string())
    );
}

#[test]
fn test_unset_ollama_config() {
    let mut config = Config::default();
    config.set_value("ollama.model", "llama3").unwrap();
    config.unset_value("ollama.model").unwrap();
    assert_eq!(config.get_value("ollama.model"), None);
    config.set_value("ollama.size", "70b").unwrap();
    config.unset_value("ollama.size").unwrap();
    assert_eq!(config.get_value("ollama.size"), None);
}

#[test]
fn test_ollama_model_getter() {
    let config = Config::default();
    assert_eq!(config.ollama_model(), "qwen3.5");

    let config = Config {
        ollama: OllamaConfig {
            model: Some("llama3".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(config.ollama_model(), "llama3");
}

#[test]
fn test_ollama_size_getter() {
    let config = Config::default();
    assert_eq!(config.ollama_size(), "9b");

    let config = Config {
        ollama: OllamaConfig {
            size: Some("70b".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(config.ollama_size(), "70b");
}

#[test]
fn test_ollama_size_for() {
    let config = Config::default();
    assert_eq!(config.ollama_size_for("small"), "2b");
    assert_eq!(config.ollama_size_for("s"), "2b");
    assert_eq!(config.ollama_size_for("medium"), "9b");
    assert_eq!(config.ollama_size_for("m"), "9b");
    assert_eq!(config.ollama_size_for("default"), "9b");
    assert_eq!(config.ollama_size_for("large"), "35b");
    assert_eq!(config.ollama_size_for("l"), "35b");
    assert_eq!(config.ollama_size_for("max"), "35b");
    assert_eq!(config.ollama_size_for("27b"), "27b"); // passthrough
}

#[test]
fn test_ollama_size_for_with_overrides() {
    let config = Config {
        ollama: OllamaConfig {
            size_small: Some("0.8b".to_string()),
            size_medium: Some("4b".to_string()),
            size_large: Some("122b".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(config.ollama_size_for("small"), "0.8b");
    assert_eq!(config.ollama_size_for("medium"), "4b");
    assert_eq!(config.ollama_size_for("large"), "122b");
}

// --- Listen config ---

#[test]
fn test_set_and_get_listen_config() {
    let mut config = Config::default();
    config.set_value("listen.format", "json").unwrap();
    assert_eq!(config.get_value("listen.format"), Some("json".to_string()));
    config
        .set_value("listen.timestamp_format", "%Y-%m-%d")
        .unwrap();
    assert_eq!(
        config.get_value("listen.timestamp_format"),
        Some("%Y-%m-%d".to_string())
    );
}

#[test]
fn test_listen_format_validation() {
    let mut config = Config::default();
    assert!(config.set_value("listen.format", "text").is_ok());
    assert!(config.set_value("listen.format", "json").is_ok());
    assert!(config.set_value("listen.format", "rich-text").is_ok());
    let result = config.set_value("listen.format", "xml");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Invalid listen format")
    );
}

#[test]
fn test_listen_format_getter() {
    let config = Config::default();
    assert_eq!(config.listen_format(), None);

    let config = Config {
        listen: ListenConfig {
            format: Some("json".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(config.listen_format(), Some("json"));
}

#[test]
fn test_listen_timestamp_format_getter() {
    let config = Config::default();
    assert_eq!(config.listen_timestamp_format(), "%H:%M:%S");

    let config = Config {
        listen: ListenConfig {
            timestamp_format: Some("%Y-%m-%d %H:%M".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(config.listen_timestamp_format(), "%Y-%m-%d %H:%M");
}

#[test]
fn test_unset_listen_config() {
    let mut config = Config::default();
    config.set_value("listen.format", "json").unwrap();
    config.unset_value("listen.format").unwrap();
    assert_eq!(config.get_value("listen.format"), None);
    config
        .set_value("listen.timestamp_format", "%H:%M")
        .unwrap();
    config.unset_value("listen.timestamp_format").unwrap();
    assert_eq!(config.get_value("listen.timestamp_format"), None);
}

#[test]
fn test_parse_ollama_and_listen_config() {
    let toml = r#"
[ollama]
model = "llama3"
size = "70b"
size_small = "1b"
size_medium = "14b"
size_large = "70b"

[listen]
format = "json"
timestamp_format = "%Y-%m-%d"
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.ollama_model(), "llama3");
    assert_eq!(config.ollama_size(), "70b");
    assert_eq!(config.listen_format(), Some("json"));
    assert_eq!(config.listen_timestamp_format(), "%Y-%m-%d");
}
