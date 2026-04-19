// provider-updated: 2026-04-05
use crate::agent::{Agent, ModelSize};
use crate::output::AgentOutput;
use crate::providers::common::CommonAgentState;
use crate::session_log::HistoricalLogAdapter;
use anyhow::Result;
use async_trait::async_trait;
use tokio::process::Command;

pub const DEFAULT_MODEL: &str = "qwen3.5";
pub const DEFAULT_SIZE: &str = "9b";

pub const AVAILABLE_SIZES: &[&str] = &["0.8b", "2b", "4b", "9b", "27b", "35b", "122b"];

pub struct Ollama {
    pub common: CommonAgentState,
    pub size: String,
}

pub struct OllamaHistoricalLogAdapter;

impl Ollama {
    pub fn new() -> Self {
        Self {
            common: CommonAgentState::new(DEFAULT_MODEL),
            size: DEFAULT_SIZE.to_string(),
        }
    }

    pub fn set_size(&mut self, size: String) {
        self.size = size;
    }

    /// Get the display string for the model (e.g., "qwen3.5:9b").
    pub fn display_model(&self) -> String {
        self.model_tag()
    }

    /// Get the full model tag (e.g., "qwen3.5:9b").
    fn model_tag(&self) -> String {
        format!("{}:{}", self.common.model, self.size)
    }

    /// Build the argument list for a run invocation.
    fn build_run_args(&self, interactive: bool, prompt: Option<&str>) -> Vec<String> {
        let mut args = vec!["run".to_string()];

        if let Some(ref format) = self.common.output_format
            && format == "json"
        {
            args.extend(["--format".to_string(), "json".to_string()]);
        }

        if !interactive {
            // --nowordwrap for clean piped output
            args.push("--nowordwrap".to_string());
        }

        args.push("--hidethinking".to_string());

        args.push(self.model_tag());

        // ollama run has no --system flag; prepend system prompt to user prompt
        let effective_prompt = match (self.common.system_prompt.is_empty(), prompt) {
            (false, Some(p)) => Some(format!("{}\n\n{}", self.common.system_prompt, p)),
            (false, None) => Some(self.common.system_prompt.clone()),
            (true, p) => p.map(String::from),
        };

        if let Some(p) = effective_prompt {
            // End option parsing before the positional prompt so prompts
            // that start with `-` / `--` aren't misread as flags.
            args.push("--".to_string());
            args.push(p);
        }

        args
    }

    /// Create a `Command` either directly or wrapped in sandbox.
    ///
    /// Ollama uses a custom sandbox implementation with shell escaping
    /// instead of the standard `build_sandbox_command`.
    fn make_command(&self, agent_args: Vec<String>) -> Command {
        if let Some(ref sb) = self.common.sandbox {
            // For ollama in sandbox, we use the shell template:
            // docker sandbox run shell <workspace> -- -c "ollama run ..."
            let shell_cmd = format!(
                "ollama {}",
                agent_args
                    .iter()
                    .map(|a| shell_escape(a))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            let mut std_cmd = std::process::Command::new("docker");
            std_cmd.args([
                "sandbox",
                "run",
                "--name",
                &sb.name,
                &sb.template,
                &sb.workspace,
                "--",
                "-c",
                &shell_cmd,
            ]);
            log::debug!(
                "Sandbox command: docker sandbox run --name {} {} {} -- -c {:?}",
                sb.name,
                sb.template,
                sb.workspace,
                shell_cmd
            );
            Command::from(std_cmd)
        } else {
            let mut cmd = Command::new("ollama");
            if let Some(ref root) = self.common.root {
                cmd.current_dir(root);
            }
            cmd.args(&agent_args);
            for (key, value) in &self.common.env_vars {
                cmd.env(key, value);
            }
            cmd
        }
    }

    async fn execute(
        &self,
        interactive: bool,
        prompt: Option<&str>,
    ) -> Result<Option<AgentOutput>> {
        let agent_args = self.build_run_args(interactive, prompt);
        log::debug!("Ollama command: ollama {}", agent_args.join(" "));
        if !self.common.system_prompt.is_empty() {
            log::debug!("Ollama system prompt: {}", self.common.system_prompt);
        }
        if let Some(p) = prompt {
            log::debug!("Ollama user prompt: {p}");
        }
        let mut cmd = self.make_command(agent_args);

        if interactive {
            CommonAgentState::run_interactive_command_with_hook(
                &mut cmd,
                "Ollama",
                self.common.on_spawn_hook.as_ref(),
            )
            .await?;
            Ok(None)
        } else {
            self.common
                .run_non_interactive_simple(&mut cmd, "Ollama")
                .await
        }
    }

    /// Resolve a size alias to the appropriate parameter size.
    pub fn size_for_model_size(size: ModelSize) -> &'static str {
        match size {
            ModelSize::Small => "2b",
            ModelSize::Medium => "9b",
            ModelSize::Large => "35b",
        }
    }
}

/// Escape a string for shell use. Wraps in single quotes if it contains special chars.
fn shell_escape(s: &str) -> String {
    if s.contains(' ')
        || s.contains('\'')
        || s.contains('"')
        || s.contains('\\')
        || s.contains('$')
        || s.contains('`')
        || s.contains('!')
    {
        format!("'{}'", s.replace('\'', "'\\''"))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
#[path = "ollama_tests.rs"]
mod tests;

impl Default for Ollama {
    fn default() -> Self {
        Self::new()
    }
}

impl HistoricalLogAdapter for OllamaHistoricalLogAdapter {
    fn backfill(&self, _root: Option<&str>) -> Result<Vec<crate::session_log::BackfilledSession>> {
        Ok(Vec::new())
    }
}

#[async_trait]
impl Agent for Ollama {
    fn name(&self) -> &str {
        "ollama"
    }

    fn default_model() -> &'static str
    where
        Self: Sized,
    {
        DEFAULT_MODEL
    }

    fn model_for_size(size: ModelSize) -> &'static str
    where
        Self: Sized,
    {
        // For ollama, model_for_size returns the size parameter, not the model name
        Self::size_for_model_size(size)
    }

    fn available_models() -> &'static [&'static str]
    where
        Self: Sized,
    {
        // Ollama accepts any model — return common sizes for validation/help
        AVAILABLE_SIZES
    }

    /// Ollama uses open model names — skip strict validation.
    fn validate_model(_model: &str, _agent_name: &str) -> Result<()>
    where
        Self: Sized,
    {
        Ok(())
    }

    crate::providers::common::impl_common_agent_setters!();

    fn set_skip_permissions(&mut self, _skip: bool) {
        // Ollama runs locally — no permission concept
        self.common.skip_permissions = true;
    }

    crate::providers::common::impl_as_any!();

    async fn run(&self, prompt: Option<&str>) -> Result<Option<AgentOutput>> {
        self.execute(false, prompt).await
    }

    async fn run_interactive(&self, prompt: Option<&str>) -> Result<()> {
        self.execute(true, prompt).await?;
        Ok(())
    }

    async fn run_resume(&self, _session_id: Option<&str>, _last: bool) -> Result<()> {
        anyhow::bail!("Ollama does not support session resume")
    }

    async fn cleanup(&self) -> Result<()> {
        Ok(())
    }
}
