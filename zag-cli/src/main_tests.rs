use super::*;

#[test]
fn test_run_resume_parses() {
    let cli = Cli::try_parse_from(["zag", "run", "--resume", "sess-123"]).unwrap();
    match cli.command {
        Commands::Run {
            resume,
            continue_session,
            prompt,
            ..
        } => {
            assert_eq!(resume.as_deref(), Some("sess-123"));
            assert!(!continue_session);
            assert!(prompt.is_none());
        }
        _ => panic!("expected run command"),
    }
}

#[test]
fn test_run_continue_parses() {
    let cli = Cli::try_parse_from(["zag", "run", "--continue"]).unwrap();
    match cli.command {
        Commands::Run {
            resume,
            continue_session,
            prompt,
            ..
        } => {
            assert!(resume.is_none());
            assert!(continue_session);
            assert!(prompt.is_none());
        }
        _ => panic!("expected run command"),
    }
}

#[test]
fn test_run_resume_rejects_prompt() {
    assert!(Cli::try_parse_from(["zag", "run", "--resume", "sess-123", "hello"]).is_err());
}

#[test]
fn test_run_resume_rejects_continue() {
    assert!(Cli::try_parse_from(["zag", "run", "--resume", "sess-123", "--continue"]).is_err());
}

// --- parse_env_vars ---

#[test]
fn test_parse_env_vars_valid() {
    let vars = vec!["FOO=bar".to_string(), "BAZ=qux".to_string()];
    let result = parse_env_vars(&vars).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], ("FOO".to_string(), "bar".to_string()));
    assert_eq!(result[1], ("BAZ".to_string(), "qux".to_string()));
}

#[test]
fn test_parse_env_vars_empty_value() {
    let vars = vec!["FOO=".to_string()];
    let result = parse_env_vars(&vars).unwrap();
    assert_eq!(result[0], ("FOO".to_string(), String::new()));
}

#[test]
fn test_parse_env_vars_value_with_equals() {
    let vars = vec!["FOO=a=b".to_string()];
    let result = parse_env_vars(&vars).unwrap();
    assert_eq!(result[0], ("FOO".to_string(), "a=b".to_string()));
}

#[test]
fn test_parse_env_vars_missing_equals() {
    let vars = vec!["INVALID".to_string()];
    assert!(parse_env_vars(&vars).is_err());
}

#[test]
fn test_env_cli_parsing() {
    let cli = Cli::try_parse_from(["zag", "run", "--env", "FOO=bar", "--env", "BAZ=qux"]).unwrap();
    match cli.command {
        Commands::Run { agent, .. } => {
            assert_eq!(agent.env_vars.len(), 2);
            assert_eq!(agent.env_vars[0], "FOO=bar");
            assert_eq!(agent.env_vars[1], "BAZ=qux");
        }
        _ => panic!("expected run command"),
    }
}

// --- resolve_provider ---

#[test]
fn test_resolve_provider_auto() {
    let result = resolve_provider(Some("auto"), None).unwrap();
    assert_eq!(result, "auto");
}

// --- capitalize ---

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
