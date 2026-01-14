use crate::agent::Agent;
use crate::claude::Claude;
use crate::codex::Codex;
use crate::config::Config;
use crate::copilot::Copilot;
use crate::gemini::Gemini;
use crate::pid;
use anyhow::{Result, bail};

pub struct AgentSession {
    pub system_prompt: Option<String>,
    pub prompt: Option<String>,
    pub agent_name: String,
    pub model_name: Option<String>,
    pub root: Option<String>,
    pub skip_permissions: bool,
    pub interactive: bool,
}

impl AgentSession {
    pub fn new(
        agent_name: impl Into<String>,
        prompt: Option<String>,
        system_prompt: Option<String>,
        model_name: Option<String>,
        root: Option<String>,
        skip_permissions: bool,
        interactive: bool,
    ) -> Self {
        Self {
            system_prompt,
            prompt,
            agent_name: agent_name.into(),
            model_name,
            root,
            skip_permissions,
            interactive,
        }
    }

    fn create_agent(&self) -> Result<Box<dyn Agent + Send>> {
        match self.agent_name.to_lowercase().as_str() {
            "codex" => Ok(Box::new(Codex::new())),
            "claude" => Ok(Box::new(Claude::new())),
            "gemini" => Ok(Box::new(Gemini::new())),
            "copilot" => Ok(Box::new(Copilot::new())),
            _ => bail!("Unknown agent: {}", self.agent_name),
        }
    }

    pub async fn run(&self) -> Result<()> {
        // Load config for defaults
        let config = Config::load(self.root.as_deref()).unwrap_or_default();

        let mut agent = self.create_agent()?;

        if let Some(ref sp) = self.system_prompt {
            agent.set_system_prompt(sp.clone());
        }

        // Use CLI model if provided, otherwise fall back to config default
        // Model input is resolved through Agent::resolve_model (handles size aliases)
        if let Some(ref model) = self.model_name {
            let resolved = resolve_model_for_agent(&self.agent_name, model);
            agent.set_model(resolved);
        } else if let Some(config_model) = config.get_model(&self.agent_name) {
            let resolved = resolve_model_for_agent(&self.agent_name, config_model);
            agent.set_model(resolved);
        }

        if let Some(ref root) = self.root {
            agent.set_root(root.clone());
        }

        // Use CLI skip_permissions if true, otherwise check config
        let skip = self.skip_permissions || config.auto_approve();
        agent.set_skip_permissions(skip);

        if self.interactive {
            agent.run_interactive(self.prompt.as_deref()).await?;
        } else {
            agent.run(self.prompt.as_deref()).await?;
        }

        agent.cleanup().await?;
        println!("Shutting down session");

        Ok(())
    }
}

/// Resolve a model input (size alias or specific name) for a given agent.
///
/// This dispatches to the appropriate Agent::resolve_model implementation.
pub fn resolve_model_for_agent(agent_name: &str, model_input: &str) -> String {
    match agent_name.to_lowercase().as_str() {
        "claude" => Claude::resolve_model(model_input),
        "codex" => Codex::resolve_model(model_input),
        "gemini" => Gemini::resolve_model(model_input),
        "copilot" => Copilot::resolve_model(model_input),
        _ => model_input.to_string(), // Unknown agent, pass through
    }
}

pub async fn run_sessions(sessions: Vec<AgentSession>, root: Option<&str>) -> Result<()> {
    // Initialize .agent directory and config on first run
    let _ = Config::init(root);

    pid::write_pid()?;

    let result = async {
        for session in sessions {
            session.run().await?;
        }
        Ok(())
    }
    .await;

    let _ = pid::remove_pid();
    result
}
