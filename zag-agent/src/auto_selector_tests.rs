use super::*;

// === extract_response tests ===

#[test]
fn test_extract_response_from_agent_output() {
    let output = crate::output::AgentOutput {
        agent: "claude".to_string(),
        session_id: "s1".to_string(),
        events: vec![],
        result: Some(r#"{"provider": "claude"}"#.to_string()),
        is_error: false,
        exit_code: None,
        error_message: None,
        total_cost_usd: None,
        usage: None,
        model: None,
        provider: None,
        log_path: None,
    };
    let response = extract_response(Some(output)).unwrap();
    assert_eq!(response, r#"{"provider": "claude"}"#);
}

#[test]
fn test_extract_response_trims_whitespace() {
    let output = crate::output::AgentOutput::from_text("test", "  claude  \n");
    let response = extract_response(Some(output)).unwrap();
    assert_eq!(response, "claude");
}

#[test]
fn test_extract_response_none_output() {
    let result = extract_response(None);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("no parseable output")
    );
}

#[test]
fn test_extract_response_no_result() {
    let output = crate::output::AgentOutput {
        agent: "test".to_string(),
        session_id: String::new(),
        events: vec![],
        result: None,
        is_error: false,
        exit_code: None,
        error_message: None,
        total_cost_usd: None,
        usage: None,
        model: None,
        provider: None,
        log_path: None,
    };
    let result = extract_response(Some(output));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("no result"));
}

// === JSON parsing tests ===

#[test]
fn test_parse_response_json_provider_and_model() {
    let result = parse_response(
        r#"{"provider": "claude", "model": "opus", "reason": "complex task"}"#,
        true,
        true,
        None,
    )
    .unwrap();
    assert_eq!(result.provider, Some("claude".to_string()));
    assert_eq!(result.model, Some("opus".to_string()));
}

#[test]
fn test_parse_response_json_provider_only() {
    let result = parse_response(
        r#"{"provider": "gemini", "reason": "large context needed"}"#,
        true,
        false,
        None,
    )
    .unwrap();
    assert_eq!(result.provider, Some("gemini".to_string()));
    assert_eq!(result.model, None);
}

#[test]
fn test_parse_response_json_model_only() {
    let result = parse_response(
        r#"{"model": "sonnet", "reason": "medium complexity"}"#,
        false,
        true,
        Some("claude"),
    )
    .unwrap();
    assert_eq!(result.provider, Some("claude".to_string()));
    assert_eq!(result.model, Some("sonnet".to_string()));
}

#[test]
fn test_parse_response_json_with_markdown_fences() {
    let result = parse_response(
        "```json\n{\"provider\": \"claude\", \"model\": \"opus\", \"reason\": \"test\"}\n```",
        true,
        true,
        None,
    )
    .unwrap();
    assert_eq!(result.provider, Some("claude".to_string()));
    assert_eq!(result.model, Some("opus".to_string()));
}

#[test]
fn test_parse_response_json_without_reason() {
    let result = parse_response(
        r#"{"provider": "codex", "model": "gpt-5.4"}"#,
        true,
        true,
        None,
    )
    .unwrap();
    assert_eq!(result.provider, Some("codex".to_string()));
    assert_eq!(result.model, Some("gpt-5.4".to_string()));
}

#[test]
fn test_parse_response_json_invalid_provider() {
    let result = parse_response(
        r#"{"provider": "unknown", "reason": "test"}"#,
        true,
        false,
        None,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("unknown provider"));
}

#[test]
fn test_parse_response_json_missing_required_field() {
    // When provider is auto but JSON has no provider field, should fall back to text parsing
    // which will also fail since it's JSON
    let result = parse_response(r#"{"model": "opus", "reason": "test"}"#, true, false, None);
    assert!(result.is_err());
}

#[test]
fn test_parse_response_json_case_insensitive_provider() {
    let result = parse_response(
        r#"{"provider": "Claude", "reason": "test"}"#,
        true,
        false,
        None,
    )
    .unwrap();
    assert_eq!(result.provider, Some("claude".to_string()));
}

// === Text fallback tests (existing) ===

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
fn test_build_mode_and_format_provider_only() {
    let (mode, response_format) = build_mode_and_format(true, false, None);
    assert_eq!(mode, "provider");
    assert!(response_format.contains("provider"));
}

#[test]
fn test_build_mode_and_format_model_only() {
    let (mode, response_format) = build_mode_and_format(false, true, Some("claude"));
    assert_eq!(mode, "model for claude");
    assert!(response_format.contains("model"));
}

#[test]
fn test_build_mode_and_format_both() {
    let (mode, response_format) = build_mode_and_format(true, true, None);
    assert_eq!(mode, "provider and model");
    assert!(response_format.contains("provider"));
    assert!(response_format.contains("model"));
}

// === Refusal detection tests ===

#[test]
fn test_is_refusal_detects_common_patterns() {
    assert!(is_refusal("I'm sorry, I can't help with that request."));
    assert!(is_refusal("I cannot assist with this type of content."));
    assert!(is_refusal("I apologize, but I'm not able to process this."));
    assert!(is_refusal("As an AI, I must decline this request."));
    assert!(is_refusal("This is against my guidelines."));
    assert!(is_refusal("I'm unable to help with that."));
    assert!(is_refusal("I won't be able to assist with that."));
}

#[test]
fn test_is_refusal_allows_valid_responses() {
    assert!(!is_refusal(r#"{"provider": "claude", "model": "opus"}"#));
    assert!(!is_refusal("claude opus"));
    assert!(!is_refusal("gemini"));
    assert!(!is_refusal(
        r#"{"provider": "codex", "reason": "fast code gen"}"#
    ));
}

#[test]
fn test_parse_response_refusal_returns_error() {
    let result = parse_response(
        "I'm sorry, but I can't assist with that kind of request.",
        true,
        true,
        None,
    );
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("declined to process"));
    assert!(err.contains("content policy"));
}

#[test]
fn test_parse_response_structured_decline() {
    let result = parse_response(
        r#"{"declined": true, "reason": "not a software engineering task"}"#,
        true,
        true,
        None,
    );
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("declined the task"));
    assert!(err.contains("not a software engineering task"));
}

#[test]
fn test_parse_response_structured_decline_without_reason() {
    let result = parse_response(r#"{"declined": true}"#, true, false, None);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("declined the task"));
    assert!(err.contains("no reason given"));
}

#[test]
fn test_parse_response_declined_false_is_not_decline() {
    // declined: false should not trigger decline handling
    let result = parse_response(
        r#"{"declined": false, "provider": "claude", "model": "opus", "reason": "test"}"#,
        true,
        true,
        None,
    );
    assert!(result.is_ok());
    let r = result.unwrap();
    assert_eq!(r.provider, Some("claude".to_string()));
    assert_eq!(r.model, Some("opus".to_string()));
}

#[test]
fn test_parse_response_refusal_with_provider_only() {
    let result = parse_response(
        "I cannot help with this task as it goes against my guidelines.",
        true,
        false,
        None,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("declined"));
}

#[test]
fn test_prompt_template_loads() {
    // Verify the prompt template is embedded and contains expected placeholders
    assert!(prompt_template().contains("{MODE}"));
    assert!(prompt_template().contains("{RESPONSE_FORMAT}"));
    assert!(prompt_template().contains("{TASK}"));
}
