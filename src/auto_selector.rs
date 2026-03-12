//! Auto-selection of provider and/or model based on task analysis.
//!
//! Runs a lightweight LLM call to analyze the user's prompt and select
//! the most suitable provider/model combination.

use crate::config::Config;
use crate::factory::AgentFactory;
use anyhow::{Result, bail};
use log::debug;

const PROMPT_TEMPLATE: &str = include_str!("../prompts/auto-selector-1_0.md");

/// Result of auto-selection.
#[derive(Debug)]
pub struct AutoResult {
    /// The selected provider (e.g., "claude", "codex", "gemini").
    pub provider: Option<String>,
    /// The selected model (e.g., "opus", "haiku", "sonnet").
    pub model: Option<String>,
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
    // Build the mode description and options
    let (mode, options) = build_mode_and_options(auto_provider, auto_model, current_provider);

    // Build the selector prompt
    let selector_prompt = PROMPT_TEMPLATE
        .replace("{MODE}", &mode)
        .replace("{OPTIONS}", &options)
        .replace("{TASK}", prompt);

    debug!("Auto-selector prompt:\n{}", selector_prompt);

    // Determine which provider/model to use for auto-selection
    let selector_provider = config
        .auto_provider()
        .unwrap_or("claude")
        .to_string();
    let selector_model = config
        .auto_model()
        .unwrap_or("haiku")
        .to_string();

    debug!(
        "Auto-selector using {} with model {}",
        selector_provider, selector_model
    );

    // Create and run the selector agent
    let spinner = crate::logging::spinner("Selecting provider/model for task...");

    let mut agent = AgentFactory::create(
        &selector_provider,
        Some("Respond with ONLY the selection, nothing else. No explanations.".to_string()),
        Some(selector_model),
        root.map(String::from),
        true, // auto-approve (selector doesn't need tools)
        vec![],
    )?;

    // Use json output to capture the result programmatically
    agent.set_output_format(Some("json".to_string()));

    let output = agent.run(Some(&selector_prompt)).await?;

    crate::logging::finish_spinner_quiet(&spinner);

    // Parse the response
    let response = extract_response(output)?;
    debug!("Auto-selector response: '{}'", response);

    parse_response(&response, auto_provider, auto_model, current_provider)
}

/// Build the mode description and options list for the prompt template.
fn build_mode_and_options(
    auto_provider: bool,
    auto_model: bool,
    current_provider: Option<&str>,
) -> (String, String) {
    if auto_provider && auto_model {
        let mode = "provider and model".to_string();
        let options = format!(
            "Respond with `<provider> <model>` (e.g., `claude opus` or `gemini gemini-2.5-flash`).\n\n{}",
            all_provider_model_options()
        );
        (mode, options)
    } else if auto_provider {
        let mode = "provider".to_string();
        let options = format!(
            "Respond with the provider name only (e.g., `claude` or `gemini`).\n\n{}",
            provider_options()
        );
        (mode, options)
    } else {
        // auto_model only
        let provider = current_provider.unwrap_or("claude");
        let mode = "model".to_string();
        let options = format!(
            "Respond with the model name only (e.g., `opus` or `sonnet`).\n\n{}",
            model_options_for_provider(provider)
        );
        (mode, options)
    }
}

/// Get descriptions of all providers.
fn provider_options() -> String {
    "### Providers\n\
     - **claude**: Best for code, reasoning, analysis, and complex tasks. Models: haiku (small/fast), sonnet (medium), opus (large/powerful)\n\
     - **codex**: OpenAI's code-focused agent. Models: gpt-5.1-codex-mini (small), gpt-5.2-codex (medium), gpt-5.1-codex-max (large)\n\
     - **gemini**: Google's model, great for large context and search. Models: gemini-2.5-flash-lite (small), gemini-2.5-flash (medium), gemini-2.5-pro (large)\n\
     - **copilot**: GitHub Copilot. Models: claude-haiku-4.5 (small), claude-sonnet-4.5 (medium), claude-opus-4.5 (large)"
        .to_string()
}

/// Get model options for a specific provider.
fn model_options_for_provider(provider: &str) -> String {
    match provider {
        "claude" => "### Models for Claude\n\
                     - **haiku**: Small, fast, cheap — for simple tasks\n\
                     - **sonnet**: Medium, balanced — for most tasks\n\
                     - **opus**: Large, most capable — for complex tasks"
            .to_string(),
        "codex" => "### Models for Codex\n\
                    - **gpt-5.1-codex-mini**: Small, fast — for simple tasks\n\
                    - **gpt-5.2-codex**: Medium, balanced — for most tasks\n\
                    - **gpt-5.1-codex-max**: Large, most capable — for complex tasks"
            .to_string(),
        "gemini" => "### Models for Gemini\n\
                     - **gemini-2.5-flash-lite**: Small, fast — for simple tasks\n\
                     - **gemini-2.5-flash**: Medium, balanced — for most tasks\n\
                     - **gemini-2.5-pro**: Large, most capable — for complex tasks"
            .to_string(),
        "copilot" => "### Models for Copilot\n\
                      - **claude-haiku-4.5**: Small, fast — for simple tasks\n\
                      - **claude-sonnet-4.5**: Medium, balanced — for most tasks\n\
                      - **claude-opus-4.5**: Large, most capable — for complex tasks"
            .to_string(),
        _ => format!("### Models\nUse appropriate model names for provider '{}'.", provider),
    }
}

/// Get all provider and model options combined.
fn all_provider_model_options() -> String {
    provider_options()
}

/// Extract the text response from the agent output.
fn extract_response(output: Option<crate::output::AgentOutput>) -> Result<String> {
    if let Some(agent_output) = output {
        if let Some(result) = agent_output.final_result() {
            return Ok(result.trim().to_string());
        }
        bail!("Auto-selector returned no result");
    }

    bail!("Auto-selector produced no parseable output. Ensure the selector agent is configured correctly.")
}

/// Parse the single-line response into an AutoResult.
fn parse_response(
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
            bail!("Auto-selector returned unparseable response: '{}'", response);
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
