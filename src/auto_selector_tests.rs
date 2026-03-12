use super::*;

#[test]
fn test_parse_response_provider_only() {
    let result = parse_response("claude", true, false, None).unwrap();
    assert_eq!(result.provider, Some("claude".to_string()));
    assert_eq!(result.model, None);
}

#[test]
fn test_parse_response_provider_and_model() {
    let result = parse_response("claude opus", true, true, None).unwrap();
    assert_eq!(result.provider, Some("claude".to_string()));
    assert_eq!(result.model, Some("opus".to_string()));
}

#[test]
fn test_parse_response_model_only() {
    let result = parse_response("sonnet", false, true, Some("claude")).unwrap();
    assert_eq!(result.provider, Some("claude".to_string()));
    assert_eq!(result.model, Some("sonnet".to_string()));
}

#[test]
fn test_parse_response_with_backticks() {
    let result = parse_response("`claude opus`", true, true, None).unwrap();
    assert_eq!(result.provider, Some("claude".to_string()));
    assert_eq!(result.model, Some("opus".to_string()));
}

#[test]
fn test_parse_response_with_extra_whitespace() {
    let result = parse_response("  gemini  gemini-2.5-pro  ", true, true, None).unwrap();
    assert_eq!(result.provider, Some("gemini".to_string()));
    assert_eq!(result.model, Some("gemini-2.5-pro".to_string()));
}

#[test]
fn test_parse_response_multiline_takes_first() {
    let result = parse_response("claude opus\nsome explanation", true, true, None).unwrap();
    assert_eq!(result.provider, Some("claude".to_string()));
    assert_eq!(result.model, Some("opus".to_string()));
}

#[test]
fn test_parse_response_invalid_provider() {
    let result = parse_response("unknown", true, false, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("unknown provider"));
}

#[test]
fn test_parse_response_empty() {
    let result = parse_response("", true, false, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("empty response"));
}

#[test]
fn test_parse_response_provider_only_when_both_auto() {
    // When both are auto but LLM only returns provider, model should be None
    let result = parse_response("codex", true, true, None).unwrap();
    assert_eq!(result.provider, Some("codex".to_string()));
    assert_eq!(result.model, None);
}

#[test]
fn test_parse_response_case_insensitive() {
    let result = parse_response("Claude", true, false, None).unwrap();
    assert_eq!(result.provider, Some("claude".to_string()));
}

#[test]
fn test_validate_provider_valid() {
    assert!(validate_provider("claude").is_ok());
    assert!(validate_provider("codex").is_ok());
    assert!(validate_provider("gemini").is_ok());
    assert!(validate_provider("copilot").is_ok());
}

#[test]
fn test_validate_provider_invalid() {
    assert!(validate_provider("unknown").is_err());
    assert!(validate_provider("gpt").is_err());
}

#[test]
fn test_build_mode_and_options_provider_only() {
    let (mode, options) = build_mode_and_options(true, false, None);
    assert_eq!(mode, "provider");
    assert!(options.contains("provider name only"));
}

#[test]
fn test_build_mode_and_options_model_only() {
    let (mode, options) = build_mode_and_options(false, true, Some("claude"));
    assert_eq!(mode, "model");
    assert!(options.contains("model name only"));
    assert!(options.contains("Claude"));
}

#[test]
fn test_build_mode_and_options_both() {
    let (mode, options) = build_mode_and_options(true, true, None);
    assert_eq!(mode, "provider and model");
    assert!(options.contains("<provider> <model>"));
}

#[test]
fn test_prompt_template_loads() {
    // Verify the prompt template is embedded and contains expected placeholders
    assert!(PROMPT_TEMPLATE.contains("{MODE}"));
    assert!(PROMPT_TEMPLATE.contains("{OPTIONS}"));
    assert!(PROMPT_TEMPLATE.contains("{TASK}"));
}
