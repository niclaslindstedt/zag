//! JSON validation utilities for `--json` and `--json-schema` output modes.

/// Strip markdown JSON fences if present (e.g., ```json ... ```).
pub fn strip_markdown_fences(text: &str) -> &str {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix("```json") {
        rest.strip_suffix("```").unwrap_or(rest).trim()
    } else if let Some(rest) = trimmed.strip_prefix("```") {
        rest.strip_suffix("```").unwrap_or(rest).trim()
    } else {
        trimmed
    }
}

/// Parse text as JSON, stripping markdown fences if present.
///
/// Returns the parsed JSON value, or an error string describing the parse failure.
pub fn validate_json(text: &str) -> Result<serde_json::Value, String> {
    let cleaned = strip_markdown_fences(text);
    serde_json::from_str(cleaned).map_err(|e| format!("Invalid JSON: {}", e))
}

/// Validate that a JSON value is a valid JSON Schema.
///
/// Returns `Ok(())` if the schema is valid, or an error string describing why it is not.
pub fn validate_schema(schema: &serde_json::Value) -> Result<(), String> {
    jsonschema::validator_for(schema)
        .map(|_| ())
        .map_err(|e| format!("Invalid JSON schema: {}", e))
}

/// Parse text as JSON and validate it against a JSON schema.
///
/// Returns the parsed JSON value, or a list of validation error strings.
pub fn validate_json_schema(
    text: &str,
    schema: &serde_json::Value,
) -> Result<serde_json::Value, Vec<String>> {
    let cleaned = strip_markdown_fences(text);
    let value: serde_json::Value =
        serde_json::from_str(cleaned).map_err(|e| vec![format!("Invalid JSON: {}", e)])?;

    let validator = jsonschema::validator_for(schema)
        .map_err(|e| vec![format!("Invalid JSON schema: {}", e)])?;

    let errors: Vec<String> = validator
        .iter_errors(&value)
        .map(|e| {
            let path = e.instance_path.to_string();
            if path.is_empty() {
                e.to_string()
            } else {
                format!("{} at {}", e, path)
            }
        })
        .collect();

    if errors.is_empty() {
        Ok(value)
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_json_valid() {
        let result = validate_json(r#"{"key": "value"}"#);
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["key"], "value");
    }

    #[test]
    fn test_validate_json_invalid() {
        let result = validate_json("not json at all");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid JSON"));
    }

    #[test]
    fn test_validate_json_with_markdown_fences() {
        let result = validate_json("```json\n{\"key\": \"value\"}\n```");
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["key"], "value");
    }

    #[test]
    fn test_validate_json_with_generic_fences() {
        let result = validate_json("```\n{\"key\": \"value\"}\n```");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_json_array() {
        let result = validate_json("[1, 2, 3]");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_json_schema_valid() {
        let schema: serde_json::Value = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": ["name"]
        });
        let result = validate_json_schema(r#"{"name": "test"}"#, &schema);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_json_schema_invalid_missing_required() {
        let schema: serde_json::Value = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": ["name"]
        });
        let result = validate_json_schema(r#"{"other": "value"}"#, &schema);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_validate_json_schema_invalid_wrong_type() {
        let schema: serde_json::Value = serde_json::json!({
            "type": "object",
            "properties": {
                "count": {"type": "integer"}
            }
        });
        let result = validate_json_schema(r#"{"count": "not a number"}"#, &schema);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_json_schema_with_fences() {
        let schema: serde_json::Value = serde_json::json!({
            "type": "object",
            "properties": {
                "items": {"type": "array"}
            }
        });
        let result = validate_json_schema("```json\n{\"items\": [1,2,3]}\n```", &schema);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_json_schema_root_error_no_dangling_at() {
        let schema: serde_json::Value = serde_json::json!({
            "type": "object",
            "required": ["languages"]
        });
        let result = validate_json_schema(r#"{"other": "value"}"#, &schema);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        // Root-level error should NOT end with " at " or " at"
        assert!(
            !errors[0].ends_with(" at"),
            "Error message has dangling 'at': {}",
            errors[0]
        );
        assert!(
            !errors[0].ends_with(" at "),
            "Error message has dangling 'at ': {}",
            errors[0]
        );
    }

    #[test]
    fn test_validate_json_schema_nested_error_includes_path() {
        let schema: serde_json::Value = serde_json::json!({
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "properties": {
                        "age": {"type": "integer"}
                    }
                }
            }
        });
        let result = validate_json_schema(r#"{"user": {"age": "not a number"}}"#, &schema);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
        assert!(
            errors[0].contains(" at "),
            "Nested error should include path: {}",
            errors[0]
        );
    }

    #[test]
    fn test_validate_schema_accepts_valid_schema() {
        let schema: serde_json::Value = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });
        assert!(validate_schema(&schema).is_ok());
    }

    #[test]
    fn test_validate_schema_rejects_invalid_schema() {
        let schema: serde_json::Value = serde_json::json!({
            "type": "not-a-real-type"
        });
        let result = validate_schema(&schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid JSON schema"));
    }

    #[test]
    fn test_strip_markdown_fences_no_fences() {
        assert_eq!(
            strip_markdown_fences(r#"{"key": "value"}"#),
            r#"{"key": "value"}"#
        );
    }

    #[test]
    fn test_strip_markdown_fences_json_fences() {
        assert_eq!(
            strip_markdown_fences("```json\n{\"key\": \"value\"}\n```"),
            "{\"key\": \"value\"}"
        );
    }

    #[test]
    fn test_strip_markdown_fences_with_whitespace() {
        assert_eq!(
            strip_markdown_fences("  ```json\n{\"key\": \"value\"}\n```  "),
            "{\"key\": \"value\"}"
        );
    }
}
