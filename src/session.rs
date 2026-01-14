use crate::agent::Agent;
use crate::claude::Claude;
use crate::codex::Codex;
use crate::copilot::Copilot;
use crate::gemini::Gemini;
use crate::pid;
use anyhow::{Result, bail};

pub struct AgentSession {
    pub system_prompt: Option<String>,
    pub prompt: String,
    pub agent_name: String,
    pub model_name: Option<String>,
    pub root: Option<String>,
    pub skip_permissions: bool,
    pub interactive: bool,
}

impl AgentSession {
    pub fn new(
        agent_name: impl Into<String>,
        prompt: impl Into<String>,
        system_prompt: Option<String>,
        model_name: Option<String>,
        root: Option<String>,
        skip_permissions: bool,
        interactive: bool,
    ) -> Self {
        Self {
            system_prompt,
            prompt: prompt.into(),
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
        let mut agent = self.create_agent()?;

        if let Some(ref sp) = self.system_prompt {
            agent.set_system_prompt(sp.clone());
        }

        if let Some(ref model) = self.model_name {
            agent.set_model(model.clone());
        }

        if let Some(ref root) = self.root {
            agent.set_root(root.clone());
        }

        agent.set_skip_permissions(self.skip_permissions);

        if self.interactive {
            agent.run_interactive(&self.prompt).await?;
        } else {
            agent.run(&self.prompt).await?;
        }

        agent.cleanup().await?;
        println!("Shutting down session");

        Ok(())
    }
}

pub async fn run_sessions(sessions: Vec<AgentSession>) -> Result<()> {
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
