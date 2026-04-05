use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

/// A feature that can be either natively supported by the provider or implemented by the wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureSupport {
    pub supported: bool,
    pub native: bool,
}

/// Session log support with completeness level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLogSupport {
    pub supported: bool,
    pub native: bool,
    /// Completeness level: "full", "partial", or absent when unsupported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completeness: Option<String>,
}

/// Size alias mappings for a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeMappings {
    pub small: String,
    pub medium: String,
    pub large: String,
}

/// All feature flags for a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub streaming_input: FeatureSupport,
    pub worktree: FeatureSupport,
    pub sandbox: FeatureSupport,
    pub system_prompt: FeatureSupport,
    pub auto_approve: FeatureSupport,
    pub review: FeatureSupport,
    pub add_dirs: FeatureSupport,
    pub max_turns: FeatureSupport,
}

/// Full capability declaration for a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapability {
    pub provider: String,
    pub default_model: String,
    pub available_models: Vec<String>,
    pub size_mappings: SizeMappings,
    pub features: Features,
}

impl FeatureSupport {
    pub fn native() -> Self {
        Self {
            supported: true,
            native: true,
        }
    }

    pub fn wrapper() -> Self {
        Self {
            supported: true,
            native: false,
        }
    }

    pub fn unsupported() -> Self {
        Self {
            supported: false,
            native: false,
        }
    }
}

impl SessionLogSupport {
    pub fn full() -> Self {
        Self {
            supported: true,
            native: true,
            completeness: Some("full".to_string()),
        }
    }

    pub fn partial() -> Self {
        Self {
            supported: true,
            native: true,
            completeness: Some("partial".to_string()),
        }
    }

    pub fn unsupported() -> Self {
        Self {
            supported: false,
            native: false,
            completeness: None,
        }
    }
}

/// Get capability declarations for a provider.
pub fn get_capability(provider: &str) -> Result<ProviderCapability> {
    use crate::agent::{Agent, ModelSize};

    match provider {
        "claude" => {
            use crate::providers::claude::{self, Claude};
            Ok(ProviderCapability {
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
            })
        }
        "codex" => {
            use crate::providers::codex::{self, Codex};
            Ok(ProviderCapability {
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
                    max_turns: FeatureSupport::native(),
                },
            })
        }
        "gemini" => {
            use crate::providers::gemini::{self, Gemini};
            Ok(ProviderCapability {
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
                    max_turns: FeatureSupport::native(),
                },
            })
        }
        "copilot" => {
            use crate::providers::copilot::{self, Copilot};
            Ok(ProviderCapability {
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
                    max_turns: FeatureSupport::native(),
                },
            })
        }
        "ollama" => {
            use crate::providers::ollama;
            Ok(ProviderCapability {
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
            })
        }
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

/// Convert a slice of string references into a Vec of owned Strings.
pub fn models_to_vec(models: &[&str]) -> Vec<String> {
    models.iter().map(|s| s.to_string()).collect()
}

#[cfg(test)]
#[path = "capability_tests.rs"]
mod tests;
