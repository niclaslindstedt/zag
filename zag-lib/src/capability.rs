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
