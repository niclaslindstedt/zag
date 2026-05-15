use super::*;

#[test]
fn template_renders_without_hint() {
    let out = build_exit_suffix(None, false, None);
    assert!(out.contains("zag ps kill self"));
    assert!(!out.contains("Expected result:"));
    assert!(!out.contains("MUST be valid JSON"));
    assert!(!out.contains("MUST validate against this schema"));
}

#[test]
fn template_renders_with_hint() {
    let out = build_exit_suffix(Some("the final answer"), false, None);
    assert!(out.contains("Expected result: the final answer"));
}

#[test]
fn template_renders_with_json_mode() {
    let out = build_exit_suffix(Some("an object"), true, None);
    assert!(out.contains("Expected result: an object"));
    assert!(out.contains("MUST be valid JSON"));
}

#[test]
fn template_renders_with_schema() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {"answer": {"type": "number"}}
    });
    let out = build_exit_suffix(Some("a number"), true, Some(&schema));
    assert!(out.contains("MUST validate against this schema"));
    assert!(out.contains("\"answer\""));
}

#[test]
fn template_renders_with_schema_but_no_json_flag() {
    // Schema implies JSON instructions even when json_mode is false
    let schema = serde_json::json!({"type": "string"});
    let out = build_exit_suffix(None, false, Some(&schema));
    assert!(out.contains("MUST be valid JSON"));
    assert!(out.contains("MUST validate against this schema"));
}

#[test]
fn validate_rejects_empty_result_with_hint() {
    let err = validate_exit_result("", Some("the answer"), false, None).unwrap_err();
    matches!(err, ExitValidationError::EmptyResult { .. });
    assert!(err.to_string().contains("hint: the answer"));
}

#[test]
fn validate_allows_empty_result_without_hint() {
    assert!(validate_exit_result("", None, false, None).is_ok());
    assert!(validate_exit_result("", Some(""), false, None).is_ok());
}

#[test]
fn validate_allows_non_empty_result_with_hint() {
    assert!(validate_exit_result("42", Some("the answer"), false, None).is_ok());
}

#[test]
fn validate_rejects_invalid_json_when_json_mode() {
    let err = validate_exit_result("not json", None, true, None).unwrap_err();
    matches!(err, ExitValidationError::InvalidJson { .. });
    assert!(err.to_string().contains("not valid JSON"));
}

#[test]
fn validate_accepts_valid_json_when_json_mode() {
    assert!(validate_exit_result(r#"{"a":1}"#, None, true, None).is_ok());
    assert!(validate_exit_result("42", None, true, None).is_ok());
    assert!(validate_exit_result("[1,2,3]", None, true, None).is_ok());
}

#[test]
fn validate_strips_markdown_fences_in_json_mode() {
    // validate_json strips fences, so this should pass
    assert!(validate_exit_result("```json\n{\"a\":1}\n```", None, true, None).is_ok());
}

#[test]
fn validate_rejects_schema_violations() {
    let schema = serde_json::json!({
        "type": "object",
        "required": ["answer"]
    });
    let err = validate_exit_result(r#"{"other":1}"#, None, true, Some(&schema)).unwrap_err();
    matches!(err, ExitValidationError::SchemaViolations { .. });
    assert!(err.to_string().contains("failed JSON-schema validation"));
}

#[test]
fn validate_accepts_schema_conforming_result() {
    let schema = serde_json::json!({
        "type": "object",
        "required": ["answer"],
        "properties": {"answer": {"type": "integer"}}
    });
    assert!(validate_exit_result(r#"{"answer":42}"#, None, true, Some(&schema)).is_ok());
}

#[test]
fn empty_result_takes_precedence_over_json_validation() {
    let err = validate_exit_result("", Some("a number"), true, None).unwrap_err();
    matches!(err, ExitValidationError::EmptyResult { .. });
}

#[test]
fn exit_hint_from_optional_collapses_empty_to_bare() {
    assert_eq!(ExitHint::from_optional(None), ExitHint::Bare);
    assert_eq!(ExitHint::from_optional(Some("".into())), ExitHint::Bare);
    assert_eq!(ExitHint::from_optional(Some("   ".into())), ExitHint::Bare);
    assert_eq!(
        ExitHint::from_optional(Some("text".into())),
        ExitHint::Provided("text".into())
    );
}

#[test]
fn exit_hint_serializes_as_string() {
    // Wire format: Bare → "", Provided(s) → s. Lets the SessionEntry
    // disk format stay flat and human-readable while Rust gets the
    // typed enum.
    let bare = serde_json::to_string(&ExitHint::Bare).unwrap();
    assert_eq!(bare, "\"\"");
    let with = serde_json::to_string(&ExitHint::Provided("foo".into())).unwrap();
    assert_eq!(with, "\"foo\"");

    let bare_back: ExitHint = serde_json::from_str("\"\"").unwrap();
    assert_eq!(bare_back, ExitHint::Bare);
    let with_back: ExitHint = serde_json::from_str("\"foo\"").unwrap();
    assert_eq!(with_back, ExitHint::Provided("foo".into()));
}

#[test]
fn exit_constraints_validate_method_matches_free_function() {
    let schema = serde_json::json!({"type": "object", "required": ["answer"]});
    let constraints = ExitConstraints {
        hint: Some(ExitHint::Provided("the answer".into())),
        json_mode: true,
        schema: Some(schema.clone()),
    };

    // Failure path: empty result.
    assert!(constraints.validate("").is_err());

    // Failure path: missing required field.
    assert!(constraints.validate(r#"{"other":1}"#).is_err());

    // Success path.
    assert!(constraints.validate(r#"{"answer":42}"#).is_ok());

    // Bare hint accepts an empty result.
    let bare = ExitConstraints {
        hint: Some(ExitHint::Bare),
        json_mode: false,
        schema: None,
    };
    assert!(bare.validate("").is_ok());
}
