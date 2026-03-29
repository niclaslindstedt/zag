pub use zag::capability::*;

use crate::agent::{Agent, ModelSize};
use crate::claude::{self, Claude};
use crate::codex::{self, Codex};
use crate::copilot::{self, Copilot};
use crate::gemini::{self, Gemini};
use crate::ollama;
use anyhow::{Result, bail};

/// Get capability declarations for a provider.
pub fn get_capability(provider: &str) -> Result<ProviderCapability> {
    match provider {
        "claude" => Ok(claude_capability()),
        "codex" => Ok(codex_capability()),
        "gemini" => Ok(gemini_capability()),
        "copilot" => Ok(copilot_capability()),
        "ollama" => Ok(ollama_capability()),
        _ => bail!(
            "No capabilities defined for provider '{}'. Available: claude, codex, gemini, copilot, ollama",
            provider
        ),
    }
}

fn claude_capability() -> ProviderCapability {
    ProviderCapability {
        provider: "claude".to_string(),
        default_model: claude::DEFAULT_MODEL.to_string(),
        available_models: models_to_vec(claude::AVAILABLE_MODELS),
        size_mappings: SizeMappings {
            small: Claude::model_for_size(ModelSize::Small).to_string(),
            medium: Claude::model_for_size(ModelSize::Medium).to_string(),
            large: Claude::model_for_size(ModelSize::Large).to_string(),
        },
        features: Features {
            interactive: FeatureSupport::native(),
            non_interactive: FeatureSupport::native(),
            resume: FeatureSupport::native(),
            resume_with_prompt: FeatureSupport::native(),
            session_logs: SessionLogSupport::full(),
            json_output: FeatureSupport::native(),
            stream_json: FeatureSupport::native(),
            json_schema: FeatureSupport::native(),
            input_format: FeatureSupport::native(),
            streaming_input: FeatureSupport::native(),
            worktree: FeatureSupport::wrapper(),
            sandbox: FeatureSupport::wrapper(),
            system_prompt: FeatureSupport::native(),
            auto_approve: FeatureSupport::native(),
            review: FeatureSupport::unsupported(),
            add_dirs: FeatureSupport::native(),
            max_turns: FeatureSupport::native(),
        },
    }
}

fn codex_capability() -> ProviderCapability {
    ProviderCapability {
        provider: "codex".to_string(),
        default_model: codex::DEFAULT_MODEL.to_string(),
        available_models: models_to_vec(codex::AVAILABLE_MODELS),
        size_mappings: SizeMappings {
            small: Codex::model_for_size(ModelSize::Small).to_string(),
            medium: Codex::model_for_size(ModelSize::Medium).to_string(),
            large: Codex::model_for_size(ModelSize::Large).to_string(),
        },
        features: Features {
            interactive: FeatureSupport::native(),
            non_interactive: FeatureSupport::native(),
            resume: FeatureSupport::native(),
            resume_with_prompt: FeatureSupport::native(),
            session_logs: SessionLogSupport::partial(),
            json_output: FeatureSupport::native(),
            stream_json: FeatureSupport::unsupported(),
            json_schema: FeatureSupport::wrapper(),
            input_format: FeatureSupport::unsupported(),
            streaming_input: FeatureSupport::unsupported(),
            worktree: FeatureSupport::wrapper(),
            sandbox: FeatureSupport::wrapper(),
            system_prompt: FeatureSupport::wrapper(),
            auto_approve: FeatureSupport::native(),
            review: FeatureSupport::native(),
            add_dirs: FeatureSupport::native(),
            max_turns: FeatureSupport::unsupported(),
        },
    }
}

fn gemini_capability() -> ProviderCapability {
    ProviderCapability {
        provider: "gemini".to_string(),
        default_model: gemini::DEFAULT_MODEL.to_string(),
        available_models: models_to_vec(gemini::AVAILABLE_MODELS),
        size_mappings: SizeMappings {
            small: Gemini::model_for_size(ModelSize::Small).to_string(),
            medium: Gemini::model_for_size(ModelSize::Medium).to_string(),
            large: Gemini::model_for_size(ModelSize::Large).to_string(),
        },
        features: Features {
            interactive: FeatureSupport::native(),
            non_interactive: FeatureSupport::native(),
            resume: FeatureSupport::native(),
            resume_with_prompt: FeatureSupport::unsupported(),
            session_logs: SessionLogSupport::full(),
            json_output: FeatureSupport::wrapper(),
            stream_json: FeatureSupport::unsupported(),
            json_schema: FeatureSupport::wrapper(),
            input_format: FeatureSupport::unsupported(),
            streaming_input: FeatureSupport::unsupported(),
            worktree: FeatureSupport::wrapper(),
            sandbox: FeatureSupport::wrapper(),
            system_prompt: FeatureSupport::wrapper(),
            auto_approve: FeatureSupport::native(),
            review: FeatureSupport::unsupported(),
            add_dirs: FeatureSupport::native(),
            max_turns: FeatureSupport::unsupported(),
        },
    }
}

fn copilot_capability() -> ProviderCapability {
    ProviderCapability {
        provider: "copilot".to_string(),
        default_model: copilot::DEFAULT_MODEL.to_string(),
        available_models: models_to_vec(copilot::AVAILABLE_MODELS),
        size_mappings: SizeMappings {
            small: Copilot::model_for_size(ModelSize::Small).to_string(),
            medium: Copilot::model_for_size(ModelSize::Medium).to_string(),
            large: Copilot::model_for_size(ModelSize::Large).to_string(),
        },
        features: Features {
            interactive: FeatureSupport::native(),
            non_interactive: FeatureSupport::native(),
            resume: FeatureSupport::native(),
            resume_with_prompt: FeatureSupport::unsupported(),
            session_logs: SessionLogSupport::full(),
            json_output: FeatureSupport::unsupported(),
            stream_json: FeatureSupport::unsupported(),
            json_schema: FeatureSupport::unsupported(),
            input_format: FeatureSupport::unsupported(),
            streaming_input: FeatureSupport::unsupported(),
            worktree: FeatureSupport::wrapper(),
            sandbox: FeatureSupport::wrapper(),
            system_prompt: FeatureSupport::wrapper(),
            auto_approve: FeatureSupport::native(),
            review: FeatureSupport::unsupported(),
            add_dirs: FeatureSupport::native(),
            max_turns: FeatureSupport::unsupported(),
        },
    }
}

fn ollama_capability() -> ProviderCapability {
    ProviderCapability {
        provider: "ollama".to_string(),
        default_model: ollama::DEFAULT_MODEL.to_string(),
        available_models: models_to_vec(ollama::AVAILABLE_SIZES),
        size_mappings: SizeMappings {
            small: "2b".to_string(),
            medium: "9b".to_string(),
            large: "35b".to_string(),
        },
        features: Features {
            interactive: FeatureSupport::native(),
            non_interactive: FeatureSupport::native(),
            resume: FeatureSupport::unsupported(),
            resume_with_prompt: FeatureSupport::unsupported(),
            session_logs: SessionLogSupport::unsupported(),
            json_output: FeatureSupport::wrapper(),
            stream_json: FeatureSupport::unsupported(),
            json_schema: FeatureSupport::wrapper(),
            input_format: FeatureSupport::unsupported(),
            streaming_input: FeatureSupport::unsupported(),
            worktree: FeatureSupport::wrapper(),
            sandbox: FeatureSupport::wrapper(),
            system_prompt: FeatureSupport::wrapper(),
            auto_approve: FeatureSupport::native(),
            review: FeatureSupport::unsupported(),
            add_dirs: FeatureSupport::unsupported(),
            max_turns: FeatureSupport::unsupported(),
        },
    }
}

#[cfg(test)]
#[path = "capability_tests.rs"]
mod tests;
