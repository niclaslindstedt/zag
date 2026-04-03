use crate::agent::Agent;
use crate::config::Config;
use crate::providers::claude::Claude;
use crate::providers::codex::Codex;
use crate::providers::copilot::Copilot;
use crate::providers::gemini::Gemini;
#[cfg(test)]
use crate::providers::mock::MockAgent;
use crate::providers::ollama::Ollama;
use anyhow::{Result, bail};
use log::debug;

pub struct AgentFactory;

impl AgentFactory {
    /// Create and configure an agent based on the provided parameters.
    ///
    /// This handles:
    /// - Loading config from ~/.zag/projects/<id>/zag.toml
    /// - Creating the appropriate agent implementation
    /// - Resolving model size aliases (small/medium/large)
    /// - Merging CLI flags with config file settings
    /// - Configuring the agent with all settings
    pub fn create(
        agent_name: &str,
        system_prompt: Option<String>,
        model: Option<String>,
        root: Option<String>,
        auto_approve: bool,
        add_dirs: Vec<String>,
    ) -> Result<Box<dyn Agent + Send + Sync>> {
        debug!("Creating agent: {}", agent_name);

        // Skip pre-flight binary check for mock agent (test only)
        #[cfg(test)]
        let skip_preflight = agent_name == "mock";
        #[cfg(not(test))]
        let skip_preflight = false;

        // Pre-flight: verify the agent CLI binary is available in PATH
        if !skip_preflight {
            crate::preflight::check_binary(agent_name)?;
        }

        // Initialize .agent directory and config on first run
        let _ = Config::init(root.as_deref());

        // Load config for defaults
        let config = Config::load(root.as_deref()).unwrap_or_default();
        debug!("Configuration loaded");

        // Create the agent
        let mut agent = Self::create_agent(agent_name)?;
        debug!("Agent instance created");

        // Configure system prompt
        if let Some(ref sp) = system_prompt {
            debug!("Setting system prompt (length: {})", sp.len());
            agent.set_system_prompt(sp.clone());
        }

        // Configure model (CLI > config > agent default)
        if let Some(model_input) = model {
            let resolved = Self::resolve_model(agent_name, &model_input);
            debug!("Model resolved from CLI: {} -> {}", model_input, resolved);
            Self::validate_model(agent_name, &resolved)?;
            agent.set_model(resolved);
        } else if let Some(config_model) = config.get_model(agent_name) {
            let resolved = Self::resolve_model(agent_name, config_model);
            debug!(
                "Model resolved from config: {} -> {}",
                config_model, resolved
            );
            Self::validate_model(agent_name, &resolved)?;
            agent.set_model(resolved);
        } else {
            debug!("Using default model for agent");
        }

        // Configure root directory
        if let Some(root_dir) = root {
            debug!("Setting root directory: {}", root_dir);
            agent.set_root(root_dir);
        }

        // Configure permissions (CLI overrides config)
        let skip = auto_approve || config.auto_approve();
        agent.set_skip_permissions(skip);

        // Configure additional directories
        if !add_dirs.is_empty() {
            agent.set_add_dirs(add_dirs);
        }

        Ok(agent)
    }

    /// Create the appropriate agent implementation based on name.
    fn create_agent(agent_name: &str) -> Result<Box<dyn Agent + Send + Sync>> {
        match agent_name.to_lowercase().as_str() {
            "codex" => Ok(Box::new(Codex::new())),
            "claude" => Ok(Box::new(Claude::new())),
            "gemini" => Ok(Box::new(Gemini::new())),
            "copilot" => Ok(Box::new(Copilot::new())),
            "ollama" => Ok(Box::new(Ollama::new())),
            #[cfg(test)]
            "mock" => Ok(Box::new(MockAgent::new())),
            _ => bail!("Unknown agent: {}", agent_name),
        }
    }

    /// Resolve a model input (size alias or specific name) for a given agent.
    fn resolve_model(agent_name: &str, model_input: &str) -> String {
        match agent_name.to_lowercase().as_str() {
            "claude" => Claude::resolve_model(model_input),
            "codex" => Codex::resolve_model(model_input),
            "gemini" => Gemini::resolve_model(model_input),
            "copilot" => Copilot::resolve_model(model_input),
            "ollama" => Ollama::resolve_model(model_input),
            #[cfg(test)]
            "mock" => MockAgent::resolve_model(model_input),
            _ => model_input.to_string(), // Unknown agent, pass through
        }
    }

    /// Validate a model for a given agent.
    fn validate_model(agent_name: &str, model: &str) -> Result<()> {
        match agent_name.to_lowercase().as_str() {
            "claude" => Claude::validate_model(model, "Claude"),
            "codex" => Codex::validate_model(model, "Codex"),
            "gemini" => Gemini::validate_model(model, "Gemini"),
            "copilot" => Copilot::validate_model(model, "Copilot"),
            "ollama" => Ollama::validate_model(model, "Ollama"),
            #[cfg(test)]
            "mock" => MockAgent::validate_model(model, "Mock"),
            _ => Ok(()), // Unknown agent, skip validation
        }
    }
}

#[cfg(test)]
#[path = "factory_tests.rs"]
mod tests;
