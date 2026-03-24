use super::*;

// --- print_manpage ---

#[test]
fn test_print_manpage_default() {
    assert!(print_manpage(None).is_ok());
}

#[test]
fn test_print_manpage_agent() {
    assert!(print_manpage(Some("agent")).is_ok());
}

#[test]
fn test_print_manpage_all_commands() {
    for cmd in &[
        "run",
        "exec",
        "review",
        "config",
        "logs",
        "capability",
        "listen",
        "man",
    ] {
        assert!(
            print_manpage(Some(cmd)).is_ok(),
            "manpage for '{}' failed",
            cmd
        );
    }
}

#[test]
fn test_print_manpage_unknown_command() {
    let result = print_manpage(Some("nonexistent"));
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("No manual entry"));
    assert!(err.contains("nonexistent"));
}

#[test]
fn test_manpage_content_has_headers() {
    // Verify embedded manpages aren't empty and have expected structure
    assert!(MAN_AGENT.contains("# agent"));
    assert!(MAN_RUN.contains("# agent run"));
    assert!(MAN_EXEC.contains("# agent exec"));
    assert!(MAN_REVIEW.contains("# agent review"));
    assert!(MAN_CONFIG.contains("# agent config"));
    assert!(MAN_MAN.contains("# agent man"));
}

#[test]
fn test_run_resume_parses() {
    let cli = Cli::try_parse_from(["agent", "run", "--resume", "sess-123"]).unwrap();
    match cli.command {
        Commands::Run {
            resume,
            continue_session,
            prompt,
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
    let cli = Cli::try_parse_from(["agent", "run", "--continue"]).unwrap();
    match cli.command {
        Commands::Run {
            resume,
            continue_session,
            prompt,
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
    assert!(Cli::try_parse_from(["agent", "run", "--resume", "sess-123", "hello"]).is_err());
}

#[test]
fn test_run_resume_rejects_continue() {
    assert!(Cli::try_parse_from(["agent", "run", "--resume", "sess-123", "--continue"]).is_err());
}

// --- wrap_prompt_for_json ---

#[test]
fn test_wrap_prompt_for_json_includes_prompt() {
    let result = wrap_prompt_for_json("list 3 colors");
    assert!(result.contains("list 3 colors"));
}

#[test]
fn test_wrap_prompt_for_json_includes_json_instruction() {
    let result = wrap_prompt_for_json("anything");
    assert!(result.contains("JSON"));
}

// --- augment_system_prompt_for_json ---

#[test]
fn test_augment_system_prompt_not_json_mode() {
    let result =
        augment_system_prompt_for_json(Some("original".to_string()), false, "codex", &None);
    assert_eq!(result, Some("original".to_string()));
}

#[test]
fn test_augment_system_prompt_claude_skipped() {
    let result =
        augment_system_prompt_for_json(Some("original".to_string()), true, "claude", &None);
    assert_eq!(result, Some("original".to_string()));
}

#[test]
fn test_augment_system_prompt_non_claude_no_schema() {
    let result = augment_system_prompt_for_json(None, true, "codex", &None);
    let prompt = result.unwrap();
    assert!(prompt.contains("valid JSON only"));
}

#[test]
fn test_augment_system_prompt_non_claude_with_schema() {
    let schema = serde_json::json!({"type": "object"});
    let result = augment_system_prompt_for_json(None, true, "gemini", &Some(schema));
    let prompt = result.unwrap();
    assert!(prompt.contains("valid JSON only"));
    assert!(prompt.contains("JSON schema"));
}

#[test]
fn test_augment_system_prompt_appends_to_existing() {
    let result =
        augment_system_prompt_for_json(Some("You are helpful.".to_string()), true, "codex", &None);
    let prompt = result.unwrap();
    assert!(prompt.starts_with("You are helpful."));
    assert!(prompt.contains("valid JSON only"));
}

// --- validate_json_output ---

#[test]
fn test_validate_json_output_valid_no_schema() {
    let result = validate_json_output(r#"{"key": "value"}"#, &None);
    assert!(result.is_ok());
}

#[test]
fn test_validate_json_output_invalid_json() {
    let result = validate_json_output("not json", &None);
    assert!(result.is_err());
    assert!(!result.unwrap_err().is_empty());
}

#[test]
fn test_validate_json_output_valid_with_schema() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {"name": {"type": "string"}},
        "required": ["name"]
    });
    let result = validate_json_output(r#"{"name": "test"}"#, &Some(schema));
    assert!(result.is_ok());
}

#[test]
fn test_validate_json_output_invalid_against_schema() {
    let schema = serde_json::json!({
        "type": "object",
        "required": ["name"]
    });
    let result = validate_json_output(r#"{"other": "value"}"#, &Some(schema));
    assert!(result.is_err());
}

// --- build_correction_prompt ---

#[test]
fn test_build_correction_prompt_single_error() {
    let prompt = build_correction_prompt(&["Invalid JSON".to_string()]);
    assert!(prompt.contains("Invalid JSON"));
    assert!(prompt.contains("valid JSON"));
}

#[test]
fn test_build_correction_prompt_multiple_errors() {
    let prompt = build_correction_prompt(&[
        "Missing field 'name'".to_string(),
        "Wrong type for 'age'".to_string(),
    ]);
    assert!(prompt.contains("Missing field 'name'"));
    assert!(prompt.contains("Wrong type for 'age'"));
}

#[test]
fn test_build_correction_prompt_empty_errors() {
    let prompt = build_correction_prompt(&[]);
    assert!(prompt.contains("valid JSON"));
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
