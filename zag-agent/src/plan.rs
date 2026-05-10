//! Library-level implementation of `zag plan`.
//!
//! Wraps a goal in the plan prompt template, runs it through a provider,
//! and either streams the result to stdout (via the agent's default output
//! handling) or captures it to a file path.
//!
//! # Example
//!
//! ```no_run
//! use zag_agent::plan::{PlanParams, run_plan};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let result = run_plan(PlanParams {
//!     provider: "claude".to_string(),
//!     goal: "Add OAuth".to_string(),
//!     output: Some("docs/oauth-plan.md".to_string()),
//!     ..PlanParams::default()
//! })
//! .await?;
//!
//! if let Some(path) = result.written_to {
//!     println!("plan written to {}", path.display());
//! }
//! # Ok(()) }
//! ```

use crate::factory::AgentFactory;
use crate::progress::{ProgressHandler, SilentProgress};
use anyhow::{Context, Result};
use log::debug;
use std::path::{Path, PathBuf};

/// Raw `prompts/plan/1_0_0.md` source, including YAML front matter.
const PLAN_TEMPLATE_SOURCE: &str = include_str!("../prompts/plan/1_0_0.md");

/// Plan prompt template (front matter stripped) — `{GOAL}`,
/// `{CONTEXT_SECTION}`, `{PROMPT}` are replaced at run time.
pub fn plan_template() -> &'static str {
    crate::prompts::strip_front_matter(PLAN_TEMPLATE_SOURCE)
}

/// Parameters for [`run_plan`].
pub struct PlanParams {
    pub provider: String,
    /// Goal to plan for.
    pub goal: String,
    /// Output path. If the path has no extension, a timestamped filename
    /// is generated inside that directory. `None` streams to stdout via
    /// the underlying agent's default output path.
    pub output: Option<String>,
    /// Additional instructions appended to the prompt.
    pub instructions: Option<String>,
    pub system_prompt: Option<String>,
    pub model: Option<String>,
    pub root: Option<String>,
    pub auto_approve: bool,
    pub add_dirs: Vec<String>,
    /// Progress handler — defaults to [`SilentProgress`].
    pub progress: Box<dyn ProgressHandler>,
}

impl Default for PlanParams {
    fn default() -> Self {
        Self {
            provider: "claude".to_string(),
            goal: String::new(),
            output: None,
            instructions: None,
            system_prompt: None,
            model: None,
            root: None,
            auto_approve: false,
            add_dirs: Vec::new(),
            progress: Box::new(SilentProgress),
        }
    }
}

/// Result of running a plan.
#[derive(Debug, Clone, Default)]
pub struct PlanResult {
    /// Captured plan text, when `output` was set (otherwise the plan was
    /// streamed to stdout by the underlying agent and nothing is captured
    /// here).
    pub text: Option<String>,
    /// Path the plan was written to, when `output` was set.
    pub written_to: Option<PathBuf>,
}

/// Render the plan prompt from [`plan_template`].
pub fn build_plan_prompt(goal: &str, instructions: Option<&str>) -> String {
    let context_section = String::new();
    let prompt_section = match instructions {
        Some(inst) => format!("## Additional Instructions\n\n{inst}"),
        None => String::new(),
    };

    plan_template()
        .replace("{GOAL}", goal)
        .replace("{CONTEXT_SECTION}", &context_section)
        .replace("{PROMPT}", &prompt_section)
}

/// Resolve a caller-supplied output path. If the input has an extension the
/// path is used verbatim; otherwise a timestamped `plan-YYYYMMDD-HHMMSS.md`
/// is generated inside that directory.
pub fn resolve_output_path(output: &str) -> PathBuf {
    let path = PathBuf::from(output);
    if path.extension().is_some() {
        path
    } else {
        let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        path.join(format!("plan-{timestamp}.md"))
    }
}

/// Validate that `path` is inside the user's home directory — used when
/// `ZAG_USER_HOME_DIR` is set (multi-user `zag serve` mode) to keep a user
/// from writing outside their sandbox. In direct CLI mode `ZAG_USER_HOME_DIR`
/// is unset and this function is a no-op.
pub fn validate_output_path(path: &Path) -> Result<()> {
    let home_dir = match std::env::var("ZAG_USER_HOME_DIR") {
        Ok(dir) => dir,
        Err(_) => return Ok(()),
    };
    let home = PathBuf::from(&home_dir);
    let canonical_home = std::fs::canonicalize(&home).unwrap_or_else(|_| home.clone());
    let check_path = if path.exists() {
        path.to_path_buf()
    } else {
        path.parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    };
    let canonical = std::fs::canonicalize(&check_path).unwrap_or_else(|_| check_path.clone());
    if !canonical.starts_with(&canonical_home) {
        anyhow::bail!(
            "Output path '{}' is outside your home directory: {}",
            path.display(),
            canonical_home.display()
        );
    }
    Ok(())
}

/// Run a plan, returning captured text and the path written (if any).
pub async fn run_plan(params: PlanParams) -> Result<PlanResult> {
    let PlanParams {
        provider,
        goal,
        output,
        instructions,
        system_prompt,
        model,
        root,
        auto_approve,
        add_dirs,
        progress,
    } = params;

    debug!("Starting plan via {provider} for goal: {goal}");

    let output_path = match output {
        Some(ref out) => {
            let resolved = resolve_output_path(out);
            validate_output_path(&resolved)?;
            Some(resolved)
        }
        None => None,
    };

    let plan_prompt = build_plan_prompt(&goal, instructions.as_deref());

    progress.on_spinner_start(&format!("Initializing {provider} for planning"));
    let mut agent = AgentFactory::create(
        &provider,
        system_prompt,
        model,
        root.clone(),
        auto_approve,
        add_dirs,
    )?;
    progress.on_spinner_finish();

    let model_name = agent.get_model().to_string();

    if output_path.is_some() {
        agent.set_capture_output(true);
    }
    progress.on_success(&format!("Plan initialized with model {model_name}"));

    let agent_output = agent.run(Some(&plan_prompt)).await?;
    agent.cleanup().await?;

    if let Some(path) = output_path {
        let plan_text = agent_output.and_then(|o| o.result).unwrap_or_default();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }
        std::fs::write(&path, &plan_text)
            .with_context(|| format!("Failed to write plan to: {}", path.display()))?;
        progress.on_success(&format!("Plan written to {}", path.display()));
        Ok(PlanResult {
            text: Some(plan_text),
            written_to: Some(path),
        })
    } else {
        Ok(PlanResult::default())
    }
}

#[cfg(test)]
#[path = "plan_tests.rs"]
mod tests;
