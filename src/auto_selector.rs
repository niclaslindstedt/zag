//! Auto-selection of provider and/or model based on task analysis.
//!
//! Runs a lightweight LLM call to analyze the user's prompt and select
//! the most suitable provider/model combination.

use crate::config::Config;
use crate::factory::AgentFactory;
use anyhow::{Result, bail};
use log::debug;
use serde::Deserialize;

const PROMPT_TEMPLATE: &str = include_str!("../prompts/auto-selector-3_1.md");

/// Result of auto-selection.
#[derive(Debug)]
pub struct AutoResult {
    /// The selected provider (e.g., "claude", "codex", "gemini").
    pub provider: Option<String>,
    /// The selected model (e.g., "opus", "haiku", "sonnet").
    pub model: Option<String>,
}

/// JSON response structure from the auto-selector LLM.
#[derive(Debug, Deserialize)]
struct AutoSelectorResponse {
    provider: Option<String>,
    model: Option<String>,
    reason: Option<String>,
    /// If true, the selector declined to route the task.
    declined: Option<bool>,
}

/// Resolve provider and/or model automatically by analyzing the task prompt.
///
/// - `prompt`: The user's task prompt to analyze.
/// - `auto_provider`: Whether the provider should be auto-selected.
/// - `auto_model`: Whether the model should be auto-selected.
/// - `current_provider`: The non-auto provider (used when only model is auto).
/// - `config`: The loaded configuration.
/// - `root`: Optional root directory for agent creation.
pub async fn resolve(
    prompt: &str,
    auto_provider: bool,
    auto_model: bool,
    current_provider: Option<&str>,
    config: &Config,
    root: Option<&str>,
) -> Result<AutoResult> {
    // Build the mode description and response format
    let (mode, response_format) =
        build_mode_and_format(auto_provider, auto_model, current_provider);

    // Build the selector prompt
    let selector_prompt = PROMPT_TEMPLATE
        .replace("{MODE}", &mode)
        .replace("{RESPONSE_FORMAT}", &response_format)
        .replace("{TASK}", prompt);

    debug!("Auto-selector prompt:\n{}", selector_prompt);

    // Determine which provider/model to use for auto-selection
    let selector_provider = config.auto_provider().unwrap_or("claude").to_string();
    let selector_model = config.auto_model().unwrap_or("sonnet").to_string();

    debug!(
        "Auto-selector using {} with model {}",
        selector_provider, selector_model
    );

    // Create and run the selector agent
    let spinner = crate::logging::spinner("Selecting provider/model for task...");

    let mut agent = AgentFactory::create(
        &selector_provider,
        Some("Respond with ONLY the JSON object, nothing else. No explanations.".to_string()),
        Some(selector_model),
        root.map(String::from),
        true, // auto-approve (selector doesn't need tools)
        vec![],
    )?;

    // Capture stdout so we can parse the response programmatically
    agent.set_capture_output(true);

    let output = agent.run(Some(&selector_prompt)).await?;

    crate::logging::finish_spinner_quiet(&spinner);

    // Parse the response
    let response = extract_response(output)?;
    debug!("Auto-selector response: '{}'", response);

    parse_response(&response, auto_provider, auto_model, current_provider)
}

/// Build the mode description and response format for the prompt template.
///
/// Returns (mode, response_format).
fn build_mode_and_format(
    auto_provider: bool,
    auto_model: bool,
    current_provider: Option<&str>,
) -> (String, String) {
    let declined_format =
        r#"If you decline the task, respond with: {"declined": true, "reason": "..."}"#;

    if auto_provider && auto_model {
        let mode = "provider and model".to_string();
        let response_format = format!(
            "Respond with ONLY a JSON object on a single line, nothing else:\n\
             {{\"provider\": \"<provider>\", \"model\": \"<size>\", \"reason\": \"...\"}}\n\n\
             {declined_format}"
        );
        (mode, response_format)
    } else if auto_provider {
        let mode = "provider".to_string();
        let response_format = format!(
            "Respond with ONLY a JSON object on a single line, nothing else:\n\
             {{\"provider\": \"<provider>\", \"reason\": \"...\"}}\n\n\
             {declined_format}"
        );
        (mode, response_format)
    } else {
        // auto_model only
        let provider = current_provider.unwrap_or("claude");
        let mode = format!("model for {}", provider);
        let response_format = format!(
            "Respond with ONLY a JSON object on a single line, nothing else:\n\
             {{\"model\": \"<model>\", \"reason\": \"...\"}}\n\n\
             {declined_format}"
        );
        (mode, response_format)
    }
}

/// Extract the text response from the agent output.
fn extract_response(output: Option<crate::output::AgentOutput>) -> Result<String> {
    if let Some(agent_output) = output {
        if let Some(result) = agent_output.final_result() {
            return Ok(result.trim().to_string());
        }
        bail!("Auto-selector returned no result");
    }

    bail!(
        "Auto-selector produced no parseable output. Ensure the selector agent is configured correctly."
    )
}

/// Strip markdown fences from a response string.
fn strip_markdown_fences(text: &str) -> &str {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix("```json") {
        rest.strip_suffix("```").unwrap_or(rest).trim()
    } else if let Some(rest) = trimmed.strip_prefix("```") {
        rest.strip_suffix("```").unwrap_or(rest).trim()
    } else {
        trimmed
    }
}

/// Check if a response looks like an LLM refusal rather than a valid selection.
fn is_refusal(response: &str) -> bool {
    let lower = response.to_lowercase();
    let refusal_patterns = [
        "i'm sorry",
        "i'm not able",
        "i cannot",
        "i can't",
        "i'm unable",
        "i apologize",
        "i must decline",
        "not appropriate",
        "i'm not going to",
        "i don't think i should",
        "i won't",
        "as an ai",
        "as a language model",
        "content policy",
        "against my guidelines",
    ];
    refusal_patterns.iter().any(|p| lower.contains(p))
}

/// Parse the response into an AutoResult.
///
/// Tries JSON parsing first, then falls back to text-based parsing for robustness.
fn parse_response(
    response: &str,
    auto_provider: bool,
    auto_model: bool,
    current_provider: Option<&str>,
) -> Result<AutoResult> {
    // Check for LLM refusal before attempting to parse
    if is_refusal(response) {
        bail!(
            "Auto-selector declined to process the prompt. The task may have been \
             filtered by the model's content policy. Try running with an explicit \
             provider and model instead of auto."
        );
    }

    // Try JSON parsing first
    let cleaned = strip_markdown_fences(response);
    if let Ok(parsed) = serde_json::from_str::<AutoSelectorResponse>(cleaned) {
        debug!("Auto-selector parsed JSON response successfully");
        if let Some(ref reason) = parsed.reason {
            debug!("Auto-selector reason: {}", reason);
        }

        // Check for structured decline
        if parsed.declined == Some(true) {
            let reason = parsed.reason.as_deref().unwrap_or("no reason given");
            bail!(
                "Auto-selector declined the task: {}. \
                 Try running with an explicit provider and model instead of auto.",
                reason
            );
        }

        return build_result_from_json(parsed, auto_provider, auto_model, current_provider);
    }

    // Fall back to text-based parsing
    debug!("Auto-selector falling back to text parsing");
    parse_response_text(response, auto_provider, auto_model, current_provider)
}

/// Build an AutoResult from a parsed JSON response.
fn build_result_from_json(
    parsed: AutoSelectorResponse,
    auto_provider: bool,
    auto_model: bool,
    current_provider: Option<&str>,
) -> Result<AutoResult> {
    if auto_provider && auto_model {
        let provider = parsed
            .provider
            .ok_or_else(|| anyhow::anyhow!("Auto-selector JSON missing 'provider' field"))?;
        let provider = validate_provider(&provider)?;
        Ok(AutoResult {
            provider: Some(provider),
            model: parsed.model,
        })
    } else if auto_provider {
        let provider = parsed
            .provider
            .ok_or_else(|| anyhow::anyhow!("Auto-selector JSON missing 'provider' field"))?;
        let provider = validate_provider(&provider)?;
        Ok(AutoResult {
            provider: Some(provider),
            model: None,
        })
    } else {
        // auto_model only
        let model = parsed
            .model
            .ok_or_else(|| anyhow::anyhow!("Auto-selector JSON missing 'model' field"))?;
        Ok(AutoResult {
            provider: current_provider.map(String::from),
            model: Some(model.to_lowercase()),
        })
    }
}

/// Parse a text-based response (fallback when JSON parsing fails).
fn parse_response_text(
    response: &str,
    auto_provider: bool,
    auto_model: bool,
    current_provider: Option<&str>,
) -> Result<AutoResult> {
    // Clean up the response - take only the first line, trim whitespace and backticks
    let cleaned = response
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .trim_matches('`')
        .trim()
        .to_lowercase();

    if cleaned.is_empty() {
        bail!("Auto-selector returned an empty response");
    }

    let parts: Vec<&str> = cleaned.split_whitespace().collect();

    if auto_provider && auto_model {
        // Expect "<provider> <model>"
        if parts.len() >= 2 {
            let provider = validate_provider(parts[0])?;
            let model = parts[1].to_string();
            Ok(AutoResult {
                provider: Some(provider),
                model: Some(model),
            })
        } else if parts.len() == 1 {
            // Just a provider, use default model
            let provider = validate_provider(parts[0])?;
            Ok(AutoResult {
                provider: Some(provider),
                model: None,
            })
        } else {
            bail!(
                "Auto-selector returned unparseable response: '{}'",
                response
            );
        }
    } else if auto_provider {
        // Expect "<provider>"
        let provider = validate_provider(parts[0])?;
        Ok(AutoResult {
            provider: Some(provider),
            model: None,
        })
    } else {
        // auto_model only - expect "<model>"
        Ok(AutoResult {
            provider: current_provider.map(String::from),
            model: Some(parts[0].to_string()),
        })
    }
}

/// Validate that a provider name is known.
fn validate_provider(name: &str) -> Result<String> {
    let normalized = name.to_lowercase();
    if Config::VALID_PROVIDERS.contains(&normalized.as_str()) {
        Ok(normalized)
    } else {
        bail!(
            "Auto-selector chose unknown provider '{}'. Available: {}",
            name,
            Config::VALID_PROVIDERS.join(", ")
        );
    }
}

#[cfg(test)]
#[path = "auto_selector_tests.rs"]
mod tests;
