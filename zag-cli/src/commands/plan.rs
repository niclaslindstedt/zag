use anyhow::{Context, Result};
use log::debug;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::factory::AgentFactory;
use crate::logging;

const PLAN_TEMPLATE: &str = include_str!("../../prompts/plan/1_0.md");

pub(crate) struct PlanParams {
    pub provider: String,
    pub goal: String,
    pub output: Option<String>,
    pub instructions: Option<String>,
    pub system_prompt: Option<String>,
    pub model: Option<String>,
    pub root: Option<String>,
    pub auto_approve: bool,
    pub add_dirs: Vec<String>,
    pub quiet: bool,
}

pub(crate) async fn run_plan(params: PlanParams) -> Result<()> {
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
        quiet,
    } = params;

    debug!("Starting plan via {provider} for goal: {goal}");

    // Resolve and validate output path early
    let output_path = match output {
        Some(ref out) => {
            let resolved = resolve_output_path(out);
            validate_output_path(&resolved)?;
            Some(resolved)
        }
        None => None,
    };

    let plan_prompt = build_plan_prompt(&goal, instructions.as_deref());

    let spinner = logging::spinner(format!("Initializing {provider} for planning"));
    let mut agent = AgentFactory::create(
        &provider,
        system_prompt,
        model,
        root.clone(),
        auto_approve,
        add_dirs,
    )?;
    logging::finish_spinner_quiet(&spinner);

    let model_name = agent.get_model().to_string();

    // If writing to a file, capture output; otherwise stream to stdout
    if output_path.is_some() {
        agent.set_capture_output(true);
    }

    if !quiet {
        eprintln!("\x1b[32m✓\x1b[0m Plan initialized with model {model_name}");
    }

    // Session logging
    let plan_session_id = uuid::Uuid::new_v4().to_string();
    let workspace_path = root.clone().or_else(|| {
        std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().to_string())
    });
    let log_metadata = crate::session_log::SessionLogMetadata {
        provider: provider.clone(),
        wrapper_session_id: plan_session_id,
        provider_session_id: None,
        workspace_path,
        command: "plan".to_string(),
        model: Some(model_name.clone()),
        resumed: false,
        backfilled: false,
    };
    let live_adapter = crate::session_log::live_adapter_for_provider(
        &provider,
        crate::session_log::LiveLogContext {
            root: root.clone(),
            provider_session_id: None,
            workspace_path: log_metadata.workspace_path.clone(),
            started_at: chrono::Utc::now(),
            is_worktree: false,
        },
        true,
    );
    let log_coordinator = crate::session_log::SessionLogCoordinator::start(
        &crate::session_log::logs_dir(root.as_deref()),
        log_metadata,
        live_adapter,
    )?;
    let _ = log_coordinator
        .writer()
        .set_global_index_dir(Config::global_base_dir());
    let log_prompt_summary = format!("plan goal={goal:?}");
    crate::session_log::record_prompt(log_coordinator.writer(), Some(&log_prompt_summary))?;

    // Register process entry
    let plan_proc_id = uuid::Uuid::new_v4().to_string();
    if let Ok(mut pstore) = zag_agent::process_store::ProcessStore::load() {
        pstore.add(zag_agent::process_store::ProcessEntry {
            id: plan_proc_id.clone(),
            pid: std::process::id(),
            session_id: None,
            provider: provider.clone(),
            model: model_name.clone(),
            command: "plan".to_string(),
            prompt: Some(log_prompt_summary.chars().take(100).collect()),
            started_at: chrono::Utc::now().to_rfc3339(),
            status: "running".to_string(),
            exit_code: None,
            exited_at: None,
            root: root.clone(),
            parent_process_id: std::env::var("ZAG_PROCESS_ID").ok(),
            parent_session_id: std::env::var("ZAG_SESSION_ID").ok(),
        });
        let _ = pstore.save();
    }

    let plan_result = agent.run(Some(&plan_prompt)).await;
    match plan_result {
        Ok(agent_output) => {
            // Write captured output to file if --output was specified
            if let Some(ref path) = output_path {
                let plan_text = agent_output.and_then(|o| o.result).unwrap_or_default();
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).with_context(|| {
                        format!("Failed to create directory: {}", parent.display())
                    })?;
                }
                std::fs::write(path, &plan_text)
                    .with_context(|| format!("Failed to write plan to: {}", path.display()))?;
                if !quiet {
                    eprintln!("\x1b[32m✓\x1b[0m Plan written to {}", path.display());
                }
            }
            log_coordinator.finish(true, None).await?;
        }
        Err(err) => {
            if let Ok(mut pstore) = zag_agent::process_store::ProcessStore::load() {
                pstore.update_status(&plan_proc_id, "killed", Some(1));
                let _ = pstore.save();
            }
            log_coordinator.finish(false, Some(err.to_string())).await?;
            return Err(err);
        }
    }

    if let Ok(mut pstore) = zag_agent::process_store::ProcessStore::load() {
        pstore.update_status(&plan_proc_id, "exited", Some(0));
        let _ = pstore.save();
    }

    Ok(())
}

fn build_plan_prompt(goal: &str, instructions: Option<&str>) -> String {
    let context_section = String::new();
    let prompt_section = match instructions {
        Some(inst) => format!("## Additional Instructions\n\n{inst}"),
        None => String::new(),
    };

    PLAN_TEMPLATE
        .replace("{GOAL}", goal)
        .replace("{CONTEXT_SECTION}", &context_section)
        .replace("{PROMPT}", &prompt_section)
}

/// Resolve an output path: if it looks like a directory (no extension), generate a filename.
fn resolve_output_path(output: &str) -> PathBuf {
    let path = PathBuf::from(output);
    if path.extension().is_some() {
        path
    } else {
        let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        path.join(format!("plan-{timestamp}.md"))
    }
}

/// Validate the output path is within the user's home directory (serve mode only).
///
/// When `ZAG_USER_HOME_DIR` is set (by `zag serve` in user-account mode),
/// the output path must be within the user's home directory. In direct CLI
/// mode this env var is unset and all paths are allowed.
fn validate_output_path(path: &Path) -> Result<()> {
    let home_dir = match std::env::var("ZAG_USER_HOME_DIR") {
        Ok(dir) => dir,
        Err(_) => return Ok(()), // no restriction in direct CLI mode
    };
    let home = PathBuf::from(&home_dir);
    let canonical_home = std::fs::canonicalize(&home).unwrap_or_else(|_| home.clone());
    // For new files, validate the parent directory exists and is within home
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

#[cfg(test)]
#[path = "plan_tests.rs"]
mod tests;
