//! JSON output mode: validation, retry logic, and prompt wrapping.

use crate::json_validation;
use anyhow::{Result, bail};
use log::debug;

const MAX_JSON_RETRIES: usize = 3;

const JSON_WRAP_TEMPLATE: &str = include_str!("../prompts/json-wrap/1_0.md");

/// Wrap a user prompt with explicit JSON instructions for non-Claude agents.
pub fn wrap_prompt_for_json(prompt: &str) -> String {
    JSON_WRAP_TEMPLATE.replace("{PROMPT}", prompt)
}

/// Handle JSON output mode: validate agent output and retry via session resume if invalid.
pub async fn handle_json_output(
    agent_output: Option<crate::output::AgentOutput>,
    agent: &(dyn crate::agent::Agent + Sync),
    schema: &Option<serde_json::Value>,
    _show_usage: bool,
    _verbose: bool,
) -> Result<()> {
    let Some(agent_out) = agent_output else {
        bail!("Agent produced no output for JSON validation");
    };

    let raw_result = agent_out
        .final_result()
        .ok_or_else(|| anyhow::anyhow!("Agent output has no result text for JSON validation"))?;
    debug!(
        "JSON mode: raw agent result ({} bytes): {}",
        raw_result.len(),
        raw_result
    );

    let result_text = json_validation::strip_markdown_fences(raw_result).to_string();
    debug!(
        "JSON mode: after fence stripping ({} bytes): {}",
        result_text.len(),
        result_text
    );

    let session_id = if !agent_out.session_id.is_empty() && agent_out.session_id != "unknown" {
        Some(agent_out.session_id.clone())
    } else {
        None
    };

    // Try validation
    if validate_json_output(&result_text, schema).is_ok() {
        // Minify JSON output
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result_text) {
            println!("{}", serde_json::to_string(&parsed)?);
        } else {
            println!("{}", result_text);
        }
        return Ok(());
    }

    // Validation failed — collect errors for retry/reporting
    let initial_errors = validate_json_output(&result_text, schema).unwrap_err();
    debug!("JSON validation failed: {:?}", initial_errors);

    let Some(sid) = session_id else {
        bail!("JSON validation failed:\n- {}", initial_errors.join("\n- "));
    };

    // Try to retry via session resume
    let mut last_errors = initial_errors;
    for attempt in 1..=MAX_JSON_RETRIES {
        debug!("JSON retry attempt {}/{}", attempt, MAX_JSON_RETRIES);

        let correction_prompt = build_correction_prompt(&last_errors);
        debug!("JSON retry correction prompt: {}", correction_prompt);

        match agent.run_resume_with_prompt(&sid, &correction_prompt).await {
            Ok(Some(retry_output)) => {
                if let Some(raw_retry_text) = retry_output.final_result() {
                    let retry_text = json_validation::strip_markdown_fences(raw_retry_text);
                    if validate_json_output(retry_text, schema).is_ok() {
                        // Minify JSON output
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(retry_text) {
                            println!("{}", serde_json::to_string(&parsed)?);
                        } else {
                            println!("{}", retry_text);
                        }
                        return Ok(());
                    }
                    last_errors = validate_json_output(retry_text, schema).unwrap_err();
                } else {
                    last_errors = vec!["Agent returned no result text".to_string()];
                }
            }
            Ok(None) => {
                last_errors = vec!["Agent produced no output on retry".to_string()];
            }
            Err(e) => {
                debug!("Resume with prompt failed: {}", e);
                break;
            }
        }
    }

    bail!(
        "JSON validation failed after {} retries. Last errors:\n- {}",
        MAX_JSON_RETRIES,
        last_errors.join("\n- ")
    )
}

/// Validate JSON output, optionally against a schema.
pub fn validate_json_output(
    text: &str,
    schema: &Option<serde_json::Value>,
) -> Result<(), Vec<String>> {
    if let Some(schema) = schema {
        json_validation::validate_json_schema(text, schema)?;
    } else {
        json_validation::validate_json(text).map_err(|e| vec![e])?;
    }
    Ok(())
}

/// Build a correction prompt for retrying invalid JSON.
pub fn build_correction_prompt(errors: &[String]) -> String {
    let error_list: String = errors
        .iter()
        .map(|e| format!("- {}", e))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Your previous response was not valid JSON. Errors:\n{}\n\nPlease respond with ONLY valid JSON. No markdown fences, no explanations.",
        error_list
    )
}

/// Augment the system prompt with JSON instructions for non-Claude agents.
pub fn augment_system_prompt_for_json(
    system_prompt: Option<String>,
    json_mode: bool,
    provider: &str,
    json_schema: &Option<serde_json::Value>,
) -> Option<String> {
    if !json_mode || provider == "claude" {
        return system_prompt;
    }

    let mut prompt = system_prompt.unwrap_or_default();
    if let Some(schema) = json_schema {
        let schema_str = serde_json::to_string_pretty(schema).unwrap_or_default();
        prompt.push_str(&format!(
            "\n\nYou MUST respond with valid JSON only. No markdown fences, no explanations. \
             Your response must conform to this JSON schema:\n{}",
            schema_str
        ));
    } else {
        prompt.push_str(
            "\n\nYou MUST respond with valid JSON only. No markdown fences, no explanations.",
        );
    }
    Some(prompt)
}
