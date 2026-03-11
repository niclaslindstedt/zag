use crate::agent::Agent;
use crate::claude::Claude;
use crate::codex::Codex;
use crate::config::Config;
use crate::copilot::Copilot;
use crate::gemini::Gemini;
use anyhow::{Result, bail};
use log::debug;

pub struct AgentFactory;

impl AgentFactory {
    /// Create and configure an agent based on the provided parameters.
    ///
    /// This handles:
    /// - Loading config from .agent/agent.toml
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
    ) -> Result<Box<dyn Agent + Send>> {
        debug!("Creating agent: {}", agent_name);

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
    fn create_agent(agent_name: &str) -> Result<Box<dyn Agent + Send>> {
        match agent_name.to_lowercase().as_str() {
            "codex" => Ok(Box::new(Codex::new())),
            "claude" => Ok(Box::new(Claude::new())),
            "gemini" => Ok(Box::new(Gemini::new())),
            "copilot" => Ok(Box::new(Copilot::new())),
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
            _ => Ok(()), // Unknown agent, skip validation
        }
    }
}
