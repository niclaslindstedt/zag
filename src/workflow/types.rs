use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A workflow definition containing phases to execute
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Workflow {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub defaults: WorkflowDefaults,
    /// Variables to resolve before workflow execution
    #[serde(default)]
    pub variables: Vec<WorkflowVariable>,
    /// Definitions injected into system prompts for all phases
    #[serde(default)]
    pub definitions: HashMap<String, DefinitionValue>,
    pub phases: Vec<Phase>,
}

/// A definition value - either a simple string or a section with nested definitions
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum DefinitionValue {
    /// A simple string definition
    Simple(String),
    /// A section containing multiple definitions
    Section(HashMap<String, String>),
}

/// A variable definition that can be resolved from various sources
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkflowVariable {
    /// Variable name used in templates (e.g., "branch" for {{branch}})
    pub name: String,
    /// Type of variable source
    #[serde(rename = "type")]
    pub var_type: VariableType,
    /// Source specification (env var name, bash command, or file path)
    pub source: String,
    /// JSON path for extracting values (only used with type=json)
    /// Supports dot-notation: .field, .nested.field, .array[0], .array[0].field
    #[serde(default)]
    pub path: Option<String>,
    /// Whether the variable must be resolved successfully (default: true)
    #[serde(default = "default_required")]
    pub required: bool,
    /// Default value if source is unavailable
    #[serde(default)]
    pub default: Option<String>,
}

/// Type of variable source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VariableType {
    /// Read from environment variable
    Env,
    /// Execute bash command and capture stdout
    Bash,
    /// Read file contents
    File,
    /// Read JSON file and extract value at path
    Json,
}

fn default_required() -> bool {
    true
}

/// Default settings for all phases in a workflow
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct WorkflowDefaults {
    #[serde(default = "default_agent")]
    pub agent: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub interactive: bool,
    #[serde(default)]
    pub skip_permissions: bool,
    /// Whether to inject workflow memories into system prompts (default: true)
    #[serde(default = "default_memory_enabled")]
    pub memory: bool,
}

fn default_memory_enabled() -> bool {
    true
}

fn default_agent() -> String {
    "claude".to_string()
}

/// A single phase in a workflow
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Phase {
    /// Unique identifier for this phase
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// How this phase should be executed
    pub execution: ExecutionConfig,
    /// Override default agent for this phase
    #[serde(default)]
    pub agent: Option<String>,
    /// Override default model for this phase
    #[serde(default)]
    pub model: Option<String>,
    /// Override interactive mode for this phase
    #[serde(default)]
    pub interactive: Option<bool>,
    /// Override skip_permissions for this phase
    #[serde(default)]
    pub skip_permissions: Option<bool>,
    /// System prompt with template variables
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// User prompt with template variables
    pub prompt: String,
    /// Output configuration
    #[serde(default)]
    pub output: Option<OutputConfig>,
    /// Phase IDs that must complete before this phase
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Parent phase ID for nested phases
    #[serde(default)]
    pub parent: Option<String>,
    /// Child phase IDs that run within this phase's iteration
    #[serde(default)]
    pub nested_phases: Vec<String>,
}

/// Configuration for how a phase executes
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExecutionConfig {
    /// Execution mode: once, iterate, or nested
    pub mode: ExecutionMode,
    /// Path to JSON array file for iteration (required if mode=iterate)
    #[serde(default)]
    pub iterate_over: Option<String>,
    /// Variable name for current item in prompt templates
    #[serde(default = "default_item_variable")]
    pub item_variable: String,
    /// Skip iteration if file is missing or empty
    #[serde(default)]
    pub skip_if_empty: bool,
}

fn default_item_variable() -> String {
    "item".to_string()
}

/// Execution mode for a phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    /// Run once
    Once,
    /// Run for each item in a JSON array
    Iterate,
}

/// Output configuration for a phase
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OutputConfig {
    /// Output filename template
    pub filename: String,
    /// Output format
    #[serde(default)]
    pub format: OutputFormat,
}

/// Output format for phase results
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
    Markdown,
}

/// Manifest tracking the state of a workflow run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunManifest {
    /// Workflow name
    pub workflow: String,
    /// Unique run identifier (timestamp-based)
    pub run_id: String,
    /// ISO 8601 timestamp when run started
    pub started_at: String,
    /// Current status of the run
    pub status: RunStatus,
    /// Currently executing phase ID
    #[serde(default)]
    pub current_phase: Option<String>,
    /// Current epic being processed (for nested workflows)
    #[serde(default)]
    pub current_epic: Option<String>,
    /// Current ticket being processed (for nested workflows)
    #[serde(default)]
    pub current_ticket: Option<String>,
    /// Current iteration index
    #[serde(default)]
    pub current_iteration: Option<usize>,
    /// Total iterations
    #[serde(default)]
    pub total_iterations: Option<usize>,
    /// Status of each phase
    #[serde(default)]
    pub phases: HashMap<String, PhaseStatus>,
}

/// Status of a workflow run
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Paused,
}

/// Status of a single phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseStatus {
    pub status: RunStatus,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub iteration: Option<usize>,
    #[serde(default)]
    pub total: Option<usize>,
    #[serde(default)]
    pub error: Option<String>,
    /// Completed iteration indices (1-based) for iterate phases
    #[serde(default)]
    pub completed_iterations: Vec<usize>,
}

impl PhaseStatus {
    pub fn pending() -> Self {
        Self {
            status: RunStatus::Pending,
            started_at: None,
            completed_at: None,
            iteration: None,
            total: None,
            error: None,
            completed_iterations: Vec::new(),
        }
    }

    pub fn in_progress() -> Self {
        Self {
            status: RunStatus::InProgress,
            started_at: Some(chrono::Utc::now().to_rfc3339()),
            completed_at: None,
            iteration: None,
            total: None,
            error: None,
            completed_iterations: Vec::new(),
        }
    }

    pub fn in_progress_preserving(existing: Option<&PhaseStatus>) -> Self {
        Self {
            status: RunStatus::InProgress,
            started_at: existing
                .and_then(|e| e.started_at.clone())
                .or_else(|| Some(chrono::Utc::now().to_rfc3339())),
            completed_at: None,
            iteration: None,
            total: None,
            error: None,
            completed_iterations: existing
                .map(|e| e.completed_iterations.clone())
                .unwrap_or_default(),
        }
    }

    pub fn completed(started_at: Option<String>) -> Self {
        Self {
            status: RunStatus::Completed,
            started_at,
            completed_at: Some(chrono::Utc::now().to_rfc3339()),
            iteration: None,
            total: None,
            error: None,
            completed_iterations: Vec::new(),
        }
    }

    pub fn failed(started_at: Option<String>, error: String) -> Self {
        Self {
            status: RunStatus::Failed,
            started_at,
            completed_at: Some(chrono::Utc::now().to_rfc3339()),
            iteration: None,
            total: None,
            error: Some(error),
            completed_iterations: Vec::new(),
        }
    }
}
