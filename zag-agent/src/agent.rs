use crate::output::AgentOutput;
use crate::sandbox::SandboxConfig;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// Callback invoked once with the OS pid of the spawned agent subprocess.
///
/// Set via [`Agent::set_on_spawn_hook`] (or
/// [`crate::builder::AgentBuilder::on_spawn`]) so callers that need to
/// act on the running child — for example, updating a process registry
/// so `zag ps kill self` can SIGTERM the agent child instead of the
/// parent zag process — can capture the pid right after spawn and
/// before the terminal wait.
///
/// The callback fires *once per spawn*, with the pid of the direct
/// provider subprocess. On retries or resumes the callback fires again
/// for the new child. `pid` is not guaranteed to still be alive by the
/// time the callback runs; use the OS to confirm before signaling.
pub type OnSpawnHook = Arc<dyn Fn(u32) + Send + Sync>;

/// Model size categories that map to agent-specific models.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelSize {
    /// Fast and lightweight model for simple tasks
    Small,
    /// Balanced model for most tasks (default)
    Medium,
    /// Most capable model for complex reasoning
    Large,
}

impl std::str::FromStr for ModelSize {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "small" | "s" => Ok(ModelSize::Small),
            "medium" | "m" | "default" => Ok(ModelSize::Medium),
            "large" | "l" | "max" => Ok(ModelSize::Large),
            _ => Err(()),
        }
    }
}

#[async_trait]
#[allow(dead_code)]
pub trait Agent {
    fn name(&self) -> &str;

    fn default_model() -> &'static str
    where
        Self: Sized;

    /// Get the model name for a given size category.
    fn model_for_size(size: ModelSize) -> &'static str
    where
        Self: Sized;

    /// Resolve a model input (either a size alias or specific model name).
    ///
    /// If the input is a size alias (small/medium/large), returns the
    /// corresponding model for this agent. Otherwise returns the input as-is.
    fn resolve_model(model_input: &str) -> String
    where
        Self: Sized,
    {
        if let Ok(size) = model_input.parse::<ModelSize>() {
            Self::model_for_size(size).to_string()
        } else {
            model_input.to_string()
        }
    }

    /// Get the list of available models for this agent.
    fn available_models() -> &'static [&'static str]
    where
        Self: Sized;

    /// Validate that a model name is supported by this agent.
    ///
    /// Returns Ok(()) if valid, or an error with available models if invalid.
    fn validate_model(model: &str, agent_name: &str) -> Result<()>
    where
        Self: Sized,
    {
        let available = Self::available_models();
        if available.contains(&model) {
            Ok(())
        } else {
            // Build error message with size aliases first
            let small = Self::model_for_size(ModelSize::Small);
            let medium = Self::model_for_size(ModelSize::Medium);
            let large = Self::model_for_size(ModelSize::Large);

            let mut models = vec![
                format!("{} (small)", small),
                format!("{} (medium)", medium),
                format!("{} (large)", large),
            ];

            // Add other available models that aren't already in the size mappings
            for m in available {
                if m != &small && m != &medium && m != &large {
                    models.push(m.to_string());
                }
            }

            anyhow::bail!(
                "Invalid model '{}' for {}. Available models: {}",
                model,
                agent_name,
                models.join(", ")
            )
        }
    }

    fn system_prompt(&self) -> &str;

    fn set_system_prompt(&mut self, prompt: String);

    fn get_model(&self) -> &str;

    fn set_model(&mut self, model: String);

    fn set_root(&mut self, root: String);

    fn set_skip_permissions(&mut self, skip: bool);

    fn set_output_format(&mut self, format: Option<String>);

    /// Enable output capture mode.
    ///
    /// When set, non-interactive `run()` pipes stdout, captures the text,
    /// and returns `Some(AgentOutput)`. Default is `false` (streams to terminal).
    /// Claude handles capture via output_format, so the default is a no-op.
    fn set_capture_output(&mut self, _capture: bool) {}

    /// Set the maximum number of agentic turns.
    fn set_max_turns(&mut self, _turns: u32) {}

    /// Set sandbox configuration for running inside a Docker sandbox.
    fn set_sandbox(&mut self, _config: SandboxConfig) {}

    /// Set additional directories for the agent to include.
    fn set_add_dirs(&mut self, dirs: Vec<String>);

    /// Set environment variables to pass to the agent subprocess.
    fn set_env_vars(&mut self, _vars: Vec<(String, String)>) {}

    /// Register a callback that fires with the OS pid of the spawned
    /// agent subprocess.
    ///
    /// Default impl is a no-op; providers that spawn an OS subprocess
    /// override this to invoke the hook after spawn. See [`OnSpawnHook`]
    /// for callback semantics.
    fn set_on_spawn_hook(&mut self, _hook: OnSpawnHook) {}

    /// Enable headless interactive mode: the provider's TUI is attached
    /// to a private pseudo-terminal instead of inheriting the user's
    /// terminal, so its output is invisible to the operator.
    ///
    /// Default impl is a no-op. Providers that build on
    /// `CommonAgentState` pick this up automatically through the shared
    /// `impl_common_agent_setters!` macro.
    fn set_headless(&mut self, _headless: bool) {}

    /// Get a reference to the concrete agent type (for downcasting).
    fn as_any_ref(&self) -> &dyn std::any::Any;

    /// Get a mutable reference to the concrete agent type (for downcasting).
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;

    /// Run the agent in non-interactive mode.
    ///
    /// Returns `Some(AgentOutput)` if the agent supports structured output
    /// (e.g., JSON mode), otherwise returns `None`.
    async fn run(&self, prompt: Option<&str>) -> Result<Option<AgentOutput>>;

    async fn run_interactive(&self, prompt: Option<&str>) -> Result<()>;

    /// Resume a previous session.
    ///
    /// If `session_id` is provided, resumes that specific session.
    /// If `last` is true, resumes the most recent session.
    /// If neither, shows a session picker or resumes the most recent.
    async fn run_resume(&self, session_id: Option<&str>, last: bool) -> Result<()>;

    /// Resume a previous session with a new prompt (for retry/correction).
    ///
    /// Returns `Some(AgentOutput)` if the agent supports structured output.
    /// Default implementation returns an error indicating unsupported operation.
    async fn run_resume_with_prompt(
        &self,
        _session_id: &str,
        _prompt: &str,
    ) -> Result<Option<AgentOutput>> {
        anyhow::bail!("Resume with prompt is not supported by this agent")
    }

    /// Lightweight startup probe used by the provider fallback mechanism.
    ///
    /// Override this in providers that can cheaply detect a broken startup
    /// state (e.g. missing auth) without consuming paid API quota. A non-Ok
    /// return value is treated as a reason to downgrade to the next provider
    /// in the tier list when the user has not pinned a provider with `-p`.
    ///
    /// The default implementation is a no-op because pre-flight PATH lookup
    /// (`preflight::check_binary`) already catches the missing-binary case.
    async fn probe(&self) -> Result<()> {
        Ok(())
    }

    async fn cleanup(&self) -> Result<()>;
}

#[cfg(test)]
#[path = "agent_tests.rs"]
mod tests;
