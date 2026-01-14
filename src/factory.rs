use crate::agent::Agent;
use crate::claude::Claude;
use crate::codex::Codex;
use crate::config::Config;
use crate::copilot::Copilot;
use crate::gemini::Gemini;
use anyhow::{Result, bail};

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
    ) -> Result<Box<dyn Agent + Send>> {
        // Initialize .agent directory and config on first run
        let _ = Config::init(root.as_deref());

        // Load config for defaults
        let config = Config::load(root.as_deref()).unwrap_or_default();

        // Create the agent
        let mut agent = Self::create_agent(agent_name)?;

        // Configure system prompt
        if let Some(sp) = system_prompt {
            agent.set_system_prompt(sp);
        }

        // Configure model (CLI > config > agent default)
        if let Some(model_input) = model {
            let resolved = Self::resolve_model(agent_name, &model_input);
            agent.set_model(resolved);
        } else if let Some(config_model) = config.get_model(agent_name) {
            let resolved = Self::resolve_model(agent_name, config_model);
            agent.set_model(resolved);
        }

        // Configure root directory
        if let Some(root_dir) = root {
            agent.set_root(root_dir);
        }

        // Configure permissions (CLI overrides config)
        let skip = auto_approve || config.auto_approve();
        agent.set_skip_permissions(skip);

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
}
