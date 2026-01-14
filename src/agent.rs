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
            anyhow::bail!(
                "Invalid model '{}' for {}. Available models: {}",
                model,
                agent_name,
                available.join(", ")
            )
        }
    }

    fn system_prompt(&self) -> &str;

    fn set_system_prompt(&mut self, prompt: String);

    fn get_model(&self) -> &str;

    fn set_model(&mut self, model: String);

    fn set_root(&mut self, root: String);

    fn set_skip_permissions(&mut self, skip: bool);

    async fn run(&self, prompt: Option<&str>) -> Result<()>;

    async fn run_interactive(&self, prompt: Option<&str>) -> Result<()>;

    async fn cleanup(&self) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::Claude;
    use crate::codex::Codex;
    use crate::copilot::Copilot;
    use crate::gemini::Gemini;

    #[test]
    fn test_model_size_from_str() {
        assert_eq!(ModelSize::from_str("small"), Some(ModelSize::Small));
        assert_eq!(ModelSize::from_str("s"), Some(ModelSize::Small));
        assert_eq!(ModelSize::from_str("SMALL"), Some(ModelSize::Small));
        assert_eq!(ModelSize::from_str("medium"), Some(ModelSize::Medium));
        assert_eq!(ModelSize::from_str("m"), Some(ModelSize::Medium));
        assert_eq!(ModelSize::from_str("large"), Some(ModelSize::Large));
        assert_eq!(ModelSize::from_str("l"), Some(ModelSize::Large));
        assert_eq!(ModelSize::from_str("max"), Some(ModelSize::Large));
        assert_eq!(ModelSize::from_str("opus"), None);
        assert_eq!(ModelSize::from_str("gpt-5"), None);
    }

    #[test]
    fn test_claude_resolve_model() {
        assert_eq!(Claude::resolve_model("small"), "haiku");
        assert_eq!(Claude::resolve_model("medium"), "sonnet");
        assert_eq!(Claude::resolve_model("large"), "opus");
        assert_eq!(Claude::resolve_model("sonnet"), "sonnet"); // passthrough
    }

    #[test]
    fn test_codex_resolve_model() {
        assert_eq!(Codex::resolve_model("small"), "gpt-5.1-codex-mini");
        assert_eq!(Codex::resolve_model("medium"), "gpt-5.2-codex");
        assert_eq!(Codex::resolve_model("large"), "gpt-5.1-codex-max");
        assert_eq!(Codex::resolve_model("gpt-5.2"), "gpt-5.2"); // passthrough
    }

    #[test]
    fn test_gemini_resolve_model() {
        assert_eq!(Gemini::resolve_model("small"), "gemini-2.5-flash-lite");
        assert_eq!(Gemini::resolve_model("medium"), "gemini-2.5-flash");
        assert_eq!(Gemini::resolve_model("large"), "gemini-2.5-pro");
        assert_eq!(Gemini::resolve_model("auto"), "auto"); // passthrough
    }

    #[test]
    fn test_copilot_resolve_model() {
        assert_eq!(Copilot::resolve_model("small"), "claude-haiku-4.5");
        assert_eq!(Copilot::resolve_model("medium"), "claude-sonnet-4.5");
        assert_eq!(Copilot::resolve_model("large"), "claude-opus-4.5");
        assert_eq!(Copilot::resolve_model("gpt-5"), "gpt-5"); // passthrough
    }

    #[test]
    fn test_short_aliases() {
        assert_eq!(Claude::resolve_model("s"), "haiku");
        assert_eq!(Claude::resolve_model("m"), "sonnet");
        assert_eq!(Claude::resolve_model("l"), "opus");
        assert_eq!(Codex::resolve_model("max"), "gpt-5.1-codex-max");
    }
}
