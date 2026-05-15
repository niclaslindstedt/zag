//! Library-level support for `--exit` interactive mode.
//!
//! When a session is launched with `--exit [<hint>]`, the user prompt is
//! augmented with instructions telling the agent to call
//! `zag ps kill self <result>` (or `zag ps kill self --file <path>`) to
//! terminate the session and submit the final result.
//!
//! This module owns the prompt template and the validation logic used
//! by `zag ps kill` to accept or reject a submitted result.

use crate::json_validation::{validate_json, validate_json_schema};

/// Raw `prompts/exit/1_0_0.md` source, including YAML front matter.
const EXIT_TEMPLATE_SOURCE: &str = include_str!("../prompts/exit/1_0_0.md");

/// Exit prompt template (front matter stripped) — `{HINT_SECTION}`,
/// `{JSON_INSTRUCTION}`, `{SCHEMA_INSTRUCTION}` are replaced at run time.
pub fn exit_template() -> &'static str {
    crate::prompts::strip_front_matter(EXIT_TEMPLATE_SOURCE)
}

/// Build the suffix appended to a user prompt when a session is launched
/// with `--exit`.
///
/// * `hint` — optional human-readable description of the expected result.
/// * `json_mode` — whether `--json` was set; the agent is told the result
///   must be valid JSON.
/// * `json_schema` — optional schema; if present, the schema is rendered
///   verbatim so the agent knows what shape to produce.
pub fn build_exit_suffix(
    hint: Option<&str>,
    json_mode: bool,
    json_schema: Option<&serde_json::Value>,
) -> String {
    let hint_section = match hint.map(str::trim).filter(|s| !s.is_empty()) {
        Some(h) => format!("Expected result: {h}\n\n"),
        None => String::new(),
    };
    let json_instruction = if json_mode || json_schema.is_some() {
        "The result you pass to `zag ps kill self` MUST be valid JSON. \
         Do not wrap it in markdown fences — pass the raw JSON string.\n\n"
            .to_string()
    } else {
        String::new()
    };
    let schema_instruction = match json_schema {
        Some(schema) => {
            let pretty = serde_json::to_string_pretty(schema).unwrap_or_default();
            format!(
                "The JSON result MUST validate against this schema:\n\n```json\n{pretty}\n```\n\n"
            )
        }
        None => String::new(),
    };
    exit_template()
        .replace("{HINT_SECTION}", &hint_section)
        .replace("{JSON_INSTRUCTION}", &json_instruction)
        .replace("{SCHEMA_INSTRUCTION}", &schema_instruction)
}

/// Reason a `zag ps kill` invocation was rejected. The CLI prints the
/// `Display` impl to stderr; the agent is expected to read the message
/// and self-correct.
#[derive(Debug)]
pub enum ExitValidationError {
    /// The session was launched with a non-empty `--exit` hint but the
    /// kill was called with an empty (or missing) result.
    EmptyResult { hint: String },
    /// `--json` was set but the result is not valid JSON.
    InvalidJson { detail: String },
    /// `--json-schema` was set and the result failed schema validation.
    SchemaViolations { errors: Vec<String> },
}

impl std::fmt::Display for ExitValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyResult { hint } => {
                write!(
                    f,
                    "Cannot terminate: a non-empty result is required (hint: {hint}). \
                     Re-run with `zag ps kill self \"<your-result>\"` or \
                     `zag ps kill self --file <path>`."
                )
            }
            Self::InvalidJson { detail } => {
                write!(
                    f,
                    "Result is not valid JSON: {detail}. The session was launched with \
                     --json, so the result must be a JSON value (object, array, string, \
                     number, boolean, or null). Do not include markdown fences."
                )
            }
            Self::SchemaViolations { errors } => {
                writeln!(
                    f,
                    "Result failed JSON-schema validation. Fix the result and call kill again:"
                )?;
                for e in errors {
                    writeln!(f, "  - {e}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ExitValidationError {}

/// Validate a result string against the constraints recorded on a session
/// at launch time. Returns `Ok(())` if the kill should proceed.
pub fn validate_exit_result(
    result: &str,
    exit_hint: Option<&str>,
    json_mode: bool,
    json_schema: Option<&serde_json::Value>,
) -> Result<(), ExitValidationError> {
    if let Some(hint) = exit_hint
        && !hint.trim().is_empty()
        && result.trim().is_empty()
    {
        return Err(ExitValidationError::EmptyResult {
            hint: hint.to_string(),
        });
    }

    if let Some(schema) = json_schema {
        if let Err(errors) = validate_json_schema(result, schema) {
            return Err(ExitValidationError::SchemaViolations { errors });
        }
    } else if json_mode && let Err(detail) = validate_json(result) {
        return Err(ExitValidationError::InvalidJson { detail });
    }

    Ok(())
}

#[cfg(test)]
#[path = "exit_mode_tests.rs"]
mod tests;
