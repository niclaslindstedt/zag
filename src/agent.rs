use crate::output::AgentOutput;
use anyhow::Result;
use async_trait::async_trait;

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

impl ModelSize {
    /// Parse a size string into ModelSize.
    ///
    /// Returns None if the string is not a recognized size alias.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "small" | "s" => Some(ModelSize::Small),
            "medium" | "m" | "default" => Some(ModelSize::Medium),
            "large" | "l" | "max" => Some(ModelSize::Large),
            _ => None,
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
        if let Some(size) = ModelSize::from_str(model_input) {
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

    /// Set additional directories for the agent to include.
    fn set_add_dirs(&mut self, dirs: Vec<String>);

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

    async fn cleanup(&self) -> Result<()>;
}

#[cfg(test)]
#[path = "agent_tests.rs"]
mod tests;
