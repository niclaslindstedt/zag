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

/// Streaming input support with mid-turn injection semantics.
///
/// Describes what happens when `StreamingSession::send_user_message` is called
/// while the agent is already producing a response on the current turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingInputSupport {
    pub supported: bool,
    pub native: bool,
    /// Mid-turn semantics when `send_user_message` is called while the agent
    /// is already producing a response. One of:
    /// - `"queue"` — message is buffered and delivered at the next turn boundary
    ///   (the current turn runs to completion before the new message is processed).
    /// - `"interrupt"` — message cancels the current turn and starts a new one
    ///   with the new input.
    /// - `"between-turns-only"` — calling mid-turn is an error or no-op; callers
    ///   must wait for the current turn to finish before sending.
    ///
    /// Absent when `supported == false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantics: Option<String>,
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
    pub streaming_input: StreamingInputSupport,
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

impl StreamingInputSupport {
    /// Mid-turn messages are queued and delivered at the next turn boundary.
    /// The currently running turn is not interrupted.
    pub fn queue() -> Self {
        Self {
            supported: true,
            native: true,
            semantics: Some("queue".to_string()),
        }
    }

    /// Mid-turn messages cancel the current turn and start a new one.
    pub fn interrupt() -> Self {
        Self {
            supported: true,
            native: true,
            semantics: Some("interrupt".to_string()),
        }
    }

    /// Messages may only be sent between turns; mid-turn sends are an error.
    pub fn between_turns_only() -> Self {
        Self {
            supported: true,
            native: true,
            semantics: Some("between-turns-only".to_string()),
        }
    }

    /// The provider does not support streaming input at all.
    pub fn unsupported() -> Self {
        Self {
            supported: false,
            native: false,
            semantics: None,
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
                    streaming_input: StreamingInputSupport::queue(),
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
                    streaming_input: StreamingInputSupport::unsupported(),
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
                    streaming_input: StreamingInputSupport::unsupported(),
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
                    streaming_input: StreamingInputSupport::unsupported(),
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
                    streaming_input: StreamingInputSupport::unsupported(),
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
            "No capabilities defined for provider '{provider}'. Available: claude, codex, gemini, copilot, ollama"
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
        _ => bail!("Unsupported format '{format}'. Available: json, yaml, toml"),
    }
}

/// Canonical list of provider names (excludes "auto" and "mock").
pub const PROVIDERS: &[&str] = &["claude", "codex", "gemini", "copilot", "ollama"];

/// List all available provider names.
pub fn list_providers() -> Vec<String> {
    PROVIDERS.iter().map(|s| s.to_string()).collect()
}

/// Get capabilities for all providers.
pub fn get_all_capabilities() -> Vec<ProviderCapability> {
    PROVIDERS
        .iter()
        .filter_map(|p| get_capability(p).ok())
        .collect()
}

/// Result of resolving a model alias.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedModel {
    pub input: String,
    pub resolved: String,
    pub is_alias: bool,
    pub provider: String,
}

/// Resolve a model name or alias for a given provider.
///
/// Size aliases (`small`/`s`, `medium`/`m`/`default`, `large`/`l`/`max`) are
/// resolved to the provider-specific model. Non-alias names pass through unchanged.
pub fn resolve_model(provider: &str, model_input: &str) -> Result<ResolvedModel> {
    use crate::agent::Agent;
    use crate::providers::{
        claude::Claude, codex::Codex, copilot::Copilot, gemini::Gemini, ollama::Ollama,
    };

    let resolved = match provider {
        "claude" => Claude::resolve_model(model_input),
        "codex" => Codex::resolve_model(model_input),
        "gemini" => Gemini::resolve_model(model_input),
        "copilot" => Copilot::resolve_model(model_input),
        "ollama" => Ollama::resolve_model(model_input),
        _ => bail!(
            "Unknown provider '{}'. Available: {}",
            provider,
            PROVIDERS.join(", ")
        ),
    };

    Ok(ResolvedModel {
        input: model_input.to_string(),
        is_alias: resolved != model_input,
        resolved,
        provider: provider.to_string(),
    })
}

/// Format a resolved model into the requested output format.
pub fn format_resolved_model(rm: &ResolvedModel, format: &str, pretty: bool) -> Result<String> {
    match format {
        "json" => {
            if pretty {
                Ok(serde_json::to_string_pretty(rm)?)
            } else {
                Ok(serde_json::to_string(rm)?)
            }
        }
        "yaml" => Ok(serde_yaml::to_string(rm)?),
        "toml" => Ok(toml::to_string_pretty(rm)?),
        _ => bail!("Unsupported format '{format}'. Available: json, yaml, toml"),
    }
}

/// Format a list of capabilities into the requested output format.
pub fn format_capabilities(
    caps: &[ProviderCapability],
    format: &str,
    pretty: bool,
) -> Result<String> {
    match format {
        "json" => {
            if pretty {
                Ok(serde_json::to_string_pretty(caps)?)
            } else {
                Ok(serde_json::to_string(caps)?)
            }
        }
        "yaml" => Ok(serde_yaml::to_string(caps)?),
        "toml" => {
            #[derive(Serialize)]
            struct Wrapper<'a> {
                providers: &'a [ProviderCapability],
            }
            Ok(toml::to_string_pretty(&Wrapper { providers: caps })?)
        }
        _ => bail!("Unsupported format '{format}'. Available: json, yaml, toml"),
    }
}

/// Format a models listing into the requested output format.
pub fn format_models(caps: &[ProviderCapability], format: &str, pretty: bool) -> Result<String> {
    #[derive(Serialize)]
    struct ModelEntry {
        provider: String,
        default_model: String,
        models: Vec<String>,
    }

    let entries: Vec<ModelEntry> = caps
        .iter()
        .map(|c| ModelEntry {
            provider: c.provider.clone(),
            default_model: c.default_model.clone(),
            models: c.available_models.clone(),
        })
        .collect();

    match format {
        "json" => {
            if pretty {
                Ok(serde_json::to_string_pretty(&entries)?)
            } else {
                Ok(serde_json::to_string(&entries)?)
            }
        }
        "yaml" => Ok(serde_yaml::to_string(&entries)?),
        "toml" => bail!("TOML does not support top-level arrays. Use json or yaml"),
        _ => bail!("Unsupported format '{format}'. Available: json, yaml, toml"),
    }
}

/// Convert a slice of string references into a Vec of owned Strings.
pub fn models_to_vec(models: &[&str]) -> Vec<String> {
    models.iter().map(|s| s.to_string()).collect()
}

#[cfg(test)]
#[path = "capability_tests.rs"]
mod tests;
