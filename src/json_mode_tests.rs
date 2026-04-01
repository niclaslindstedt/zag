use super::*;

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
