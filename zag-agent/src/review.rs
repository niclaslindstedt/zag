//! Library-level implementation of `zag review`.
//!
//! Gathers a git diff (staged/unstaged/untracked, against a base branch, or
//! at a specific commit), wraps it in the review prompt template, and runs
//! it through a provider. For Codex, the provider's native `codex review`
//! command is used instead; every other provider uses the generic prompt
//! path.
//!
//! # Example
//!
//! ```no_run
//! use zag_agent::review::{ReviewParams, run_review};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let output = run_review(ReviewParams {
//!     provider: "claude".to_string(),
//!     uncommitted: true,
//!     ..ReviewParams::default()
//! })
//! .await?;
//! println!("{}", output.map(|o| o.result.unwrap_or_default()).unwrap_or_default());
//! # Ok(()) }
//! ```

use crate::factory::AgentFactory;
use crate::output::AgentOutput;
use crate::progress::{ProgressHandler, SilentProgress};
use crate::providers::codex::Codex;
use anyhow::{Result, bail};
use log::debug;
use std::process::Command;

/// Review prompt template — `{DIFF}`, `{TITLE_SECTION}`, `{PROMPT}` are
/// replaced at run time.
pub const REVIEW_TEMPLATE: &str = include_str!("../prompts/review/1_0.md");

/// Parameters for [`run_review`].
pub struct ReviewParams {
    /// Provider name (e.g. `"claude"`, `"codex"`).
    pub provider: String,
    /// Include staged, unstaged, and untracked changes.
    pub uncommitted: bool,
    /// Diff against this base branch (e.g. `Some("main")`).
    pub base: Option<String>,
    /// Review the diff of a specific commit.
    pub commit: Option<String>,
    /// Optional title to render in the review prompt.
    pub title: Option<String>,
    /// Free-form reviewer instructions appended to the prompt.
    pub prompt: Option<String>,
    /// System prompt override.
    pub system_prompt: Option<String>,
    /// Model override.
    pub model: Option<String>,
    /// Working directory.
    pub root: Option<String>,
    /// Skip permission prompts.
    pub auto_approve: bool,
    /// Additional directories to include.
    pub add_dirs: Vec<String>,
    /// Progress handler for status / spinner callbacks. Defaults to
    /// [`SilentProgress`].
    pub progress: Box<dyn ProgressHandler>,
}

impl Default for ReviewParams {
    fn default() -> Self {
        Self {
            provider: "claude".to_string(),
            uncommitted: false,
            base: None,
            commit: None,
            title: None,
            prompt: None,
            system_prompt: None,
            model: None,
            root: None,
            auto_approve: false,
            add_dirs: Vec::new(),
            progress: Box::new(SilentProgress),
        }
    }
}

/// Gather `git diff` content for the given review targets. The returned
/// string concatenates all requested diffs; bails if nothing matched.
///
/// `uncommitted` = `true` captures staged, unstaged, AND untracked files.
pub fn gather_diff(
    uncommitted: bool,
    base: Option<&str>,
    commit: Option<&str>,
    root: Option<&str>,
) -> Result<String> {
    let dir = root.unwrap_or(".");
    let mut diffs = Vec::new();

    if uncommitted {
        let output = Command::new("git")
            .args(["diff", "HEAD"])
            .current_dir(dir)
            .output()?;
        let diff = String::from_utf8_lossy(&output.stdout).to_string();
        if !diff.trim().is_empty() {
            diffs.push(diff);
        }

        // Also capture untracked files as pseudo-diffs so the reviewer sees new files.
        let untracked = Command::new("git")
            .args(["ls-files", "--others", "--exclude-standard"])
            .current_dir(dir)
            .output()?;
        let untracked_output = String::from_utf8_lossy(&untracked.stdout).to_string();
        let files: Vec<&str> = untracked_output.lines().filter(|l| !l.is_empty()).collect();
        for file in files {
            let content = Command::new("git")
                .args(["diff", "--no-index", "/dev/null", file])
                .current_dir(dir)
                .output()?;
            let d = String::from_utf8_lossy(&content.stdout).to_string();
            if !d.trim().is_empty() {
                diffs.push(d);
            }
        }
    }

    if let Some(base_branch) = base {
        let output = Command::new("git")
            .args(["diff", &format!("{base_branch}...HEAD")])
            .current_dir(dir)
            .output()?;
        let diff = String::from_utf8_lossy(&output.stdout).to_string();
        if !diff.trim().is_empty() {
            diffs.push(diff);
        }
    }

    if let Some(sha) = commit {
        let output = Command::new("git")
            .args(["show", sha, "--format="])
            .current_dir(dir)
            .output()?;
        let diff = String::from_utf8_lossy(&output.stdout).to_string();
        if !diff.trim().is_empty() {
            diffs.push(diff);
        }
    }

    let combined = diffs.join("\n");
    if combined.trim().is_empty() {
        bail!("No diff content found for the specified review target");
    }
    Ok(combined)
}

/// Render a review prompt from [`REVIEW_TEMPLATE`] with the given diff,
/// optional title, and optional reviewer prompt.
pub fn build_review_prompt(diff: &str, title: Option<&str>, user_prompt: Option<&str>) -> String {
    let title_section = match title {
        Some(t) => format!("## Review Title\n\n{t}"),
        None => String::new(),
    };
    let prompt_section = user_prompt.unwrap_or("");

    REVIEW_TEMPLATE
        .replace("{DIFF}", diff)
        .replace("{TITLE_SECTION}", &title_section)
        .replace("{PROMPT}", prompt_section)
}

/// Run a review, returning the structured agent output (or `None` when the
/// provider doesn't surface a result — e.g. the codex-native path).
pub async fn run_review(params: ReviewParams) -> Result<Option<AgentOutput>> {
    if !params.uncommitted && params.base.is_none() && params.commit.is_none() {
        bail!("Review requires at least one of: uncommitted=true, base=<branch>, commit=<sha>");
    }

    if params.provider == "codex" {
        run_codex_review(params).await.map(|_| None)
    } else {
        run_generic_review(params).await.map(Some)
    }
}

async fn run_generic_review(params: ReviewParams) -> Result<AgentOutput> {
    let ReviewParams {
        provider,
        uncommitted,
        base,
        commit,
        title,
        prompt,
        system_prompt,
        model,
        root,
        auto_approve,
        add_dirs,
        progress,
    } = params;

    debug!(
        "Starting code review via {provider} (uncommitted={uncommitted}, base={base:?}, commit={commit:?})"
    );

    let diff = gather_diff(
        uncommitted,
        base.as_deref(),
        commit.as_deref(),
        root.as_deref(),
    )?;
    let review_prompt = build_review_prompt(&diff, title.as_deref(), prompt.as_deref());

    progress.on_spinner_start(&format!("Initializing {provider} for review"));
    let agent = AgentFactory::create(
        &provider,
        system_prompt,
        model,
        root.clone(),
        auto_approve,
        add_dirs,
    )?;
    progress.on_spinner_finish();

    let model_name = agent.get_model().to_string();
    progress.on_success(&format!("Review initialized with model {model_name}"));

    let output = agent.run(Some(&review_prompt)).await?;
    agent.cleanup().await?;
    Ok(output.unwrap_or_else(|| AgentOutput::from_text(&provider, "")))
}

async fn run_codex_review(params: ReviewParams) -> Result<()> {
    let ReviewParams {
        uncommitted,
        base,
        commit,
        title,
        system_prompt,
        model,
        root,
        auto_approve,
        add_dirs,
        progress,
        ..
    } = params;

    debug!(
        "Starting code review via Codex (uncommitted={uncommitted}, base={base:?}, commit={commit:?})"
    );

    progress.on_spinner_start("Initializing Codex for review");
    let mut agent = AgentFactory::create(
        "codex",
        system_prompt,
        model,
        root.clone(),
        auto_approve,
        add_dirs,
    )?;
    progress.on_spinner_finish();

    let model_name = agent.get_model().to_string();
    progress.on_success(&format!("Review initialized with model {model_name}"));

    let codex = agent
        .as_any_mut()
        .downcast_mut::<Codex>()
        .expect("Failed to get Codex agent for review");

    codex
        .review(
            uncommitted,
            base.as_deref(),
            commit.as_deref(),
            title.as_deref(),
        )
        .await
}

#[cfg(test)]
#[path = "review_tests.rs"]
mod tests;
