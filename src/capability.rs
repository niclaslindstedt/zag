use crate::agent::{Agent, ModelSize};
use crate::claude::{self, Claude};
use crate::codex::{self, Codex};
use crate::copilot::{self, Copilot};
use crate::gemini::{self, Gemini};
use crate::ollama;
use anyhow::{Result, bail};
use serde::Serialize;

/// A feature that can be either natively supported by the provider or implemented by the wrapper.
#[derive(Debug, Clone, Serialize)]
pub struct FeatureSupport {
    pub supported: bool,
    pub native: bool,
}

/// Session log support with completeness level.
#[derive(Debug, Clone, Serialize)]
pub struct SessionLogSupport {
    pub supported: bool,
    pub native: bool,
    /// Completeness level: "full", "partial", or absent when unsupported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completeness: Option<String>,
}

/// Size alias mappings for a provider.
#[derive(Debug, Clone, Serialize)]
pub struct SizeMappings {
    pub small: String,
    pub medium: String,
    pub large: String,
}

/// All feature flags for a provider.
#[derive(Debug, Clone, Serialize)]
pub struct Features {
    pub interactive: FeatureSupport,
    pub non_interactive: FeatureSupport,
    pub resume: FeatureSupport,
    pub resume_with_prompt: FeatureSupport,
    pub session_logs: SessionLogSupport,
    pub json_output: FeatureSupport,
    pub stream_json: FeatureSupport,
    pub json_schema: FeatureSupport,
    pub input_format: FeatureSupport,
    pub worktree: FeatureSupport,
    pub sandbox: FeatureSupport,
    pub system_prompt: FeatureSupport,
    pub auto_approve: FeatureSupport,
    pub review: FeatureSupport,
    pub add_dirs: FeatureSupport,
}

/// Full capability declaration for a provider.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderCapability {
    pub provider: String,
    pub default_model: String,
    pub available_models: Vec<String>,
    pub size_mappings: SizeMappings,
    pub features: Features,
}

impl FeatureSupport {
    fn native() -> Self {
        Self {
            supported: true,
            native: true,
        }
    }

    fn wrapper() -> Self {
        Self {
            supported: true,
            native: false,
        }
    }

    fn unsupported() -> Self {
        Self {
            supported: false,
            native: false,
        }
    }
}

impl SessionLogSupport {
    fn full() -> Self {
        Self {
            supported: true,
            native: true,
            completeness: Some("full".to_string()),
        }
    }

    fn partial() -> Self {
        Self {
            supported: true,
            native: true,
            completeness: Some("partial".to_string()),
        }
    }

    fn unsupported() -> Self {
        Self {
            supported: false,
            native: false,
            completeness: None,
        }
    }
}

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

/// Format a capability struct into the requested output format.
pub fn format_capability(cap: &ProviderCapability, format: &str, pretty: bool) -> Result<String> {
    match format {
        "json" => {
            if pretty {
                Ok(serde_json::to_string_pretty(cap)?)
            } else {
                Ok(serde_json::to_string(cap)?)
            }
        }
        "yaml" => Ok(serde_yaml::to_string(cap)?),
        "toml" => Ok(toml::to_string_pretty(cap)?),
        _ => bail!(
            "Unsupported format '{}'. Available: json, yaml, toml",
            format
        ),
    }
}

fn models_to_vec(models: &[&str]) -> Vec<String> {
    models.iter().map(|s| s.to_string()).collect()
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
            worktree: FeatureSupport::wrapper(),
            sandbox: FeatureSupport::wrapper(),
            system_prompt: FeatureSupport::native(),
            auto_approve: FeatureSupport::native(),
            review: FeatureSupport::unsupported(),
            add_dirs: FeatureSupport::native(),
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
            worktree: FeatureSupport::wrapper(),
            sandbox: FeatureSupport::wrapper(),
            system_prompt: FeatureSupport::wrapper(),
            auto_approve: FeatureSupport::native(),
            review: FeatureSupport::native(),
            add_dirs: FeatureSupport::native(),
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
            worktree: FeatureSupport::wrapper(),
            sandbox: FeatureSupport::wrapper(),
            system_prompt: FeatureSupport::wrapper(),
            auto_approve: FeatureSupport::native(),
            review: FeatureSupport::unsupported(),
            add_dirs: FeatureSupport::native(),
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
            worktree: FeatureSupport::wrapper(),
            sandbox: FeatureSupport::wrapper(),
            system_prompt: FeatureSupport::wrapper(),
            auto_approve: FeatureSupport::native(),
            review: FeatureSupport::unsupported(),
            add_dirs: FeatureSupport::native(),
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
            worktree: FeatureSupport::wrapper(),
            sandbox: FeatureSupport::wrapper(),
            system_prompt: FeatureSupport::wrapper(),
            auto_approve: FeatureSupport::native(),
            review: FeatureSupport::unsupported(),
            add_dirs: FeatureSupport::unsupported(),
        },
    }
}

#[cfg(test)]
#[path = "capability_tests.rs"]
mod tests;
