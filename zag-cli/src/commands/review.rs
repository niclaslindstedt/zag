use anyhow::{Result, bail};
use log::debug;
use std::process::Command;

use crate::config::Config;
use crate::factory::AgentFactory;
use crate::logging;

const REVIEW_TEMPLATE: &str = include_str!("../../prompts/review/1_0.md");

pub(crate) struct ReviewParams {
    pub provider: String,
    pub uncommitted: bool,
    pub base: Option<String>,
    pub commit: Option<String>,
    pub title: Option<String>,
    pub prompt: Option<String>,
    pub system_prompt: Option<String>,
    pub model: Option<String>,
    pub root: Option<String>,
    pub auto_approve: bool,
    pub add_dirs: Vec<String>,
    pub quiet: bool,
}

pub(crate) async fn run_review(params: ReviewParams) -> Result<()> {
    if !params.uncommitted && params.base.is_none() && params.commit.is_none() {
        bail!("Review requires at least one of: --uncommitted, --base <BRANCH>, --commit <SHA>");
    }

    if params.provider == "codex" {
        return run_codex_review(params).await;
    }

    run_generic_review(params).await
}

/// Gather git diff content based on the review flags.
fn gather_diff(
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

/// Build a review prompt from the template, injecting diff, title, and user prompt.
fn build_review_prompt(diff: &str, title: Option<&str>, user_prompt: Option<&str>) -> String {
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

/// Generic prompt-based review for non-Codex providers.
async fn run_generic_review(params: ReviewParams) -> Result<()> {
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
        quiet,
    } = params;

    debug!(
        "Starting code review via {} (uncommitted={}, base={:?}, commit={:?})",
        provider, uncommitted, base, commit
    );

    let diff = gather_diff(
        uncommitted,
        base.as_deref(),
        commit.as_deref(),
        root.as_deref(),
    )?;
    let review_prompt = build_review_prompt(&diff, title.as_deref(), prompt.as_deref());

    let spinner = logging::spinner(format!("Initializing {provider} for review"));
    let agent = AgentFactory::create(
        &provider,
        system_prompt,
        model,
        root.clone(),
        auto_approve,
        add_dirs,
    )?;
    logging::finish_spinner_quiet(&spinner);

    let model_name = agent.get_model().to_string();
    if !quiet {
        println!(
            "\x1b[32m✓\x1b[0m Review initialized with model {}",
            model_name
        );
    }

    // Session logging
    let review_session_id = uuid::Uuid::new_v4().to_string();
    let workspace_path = root.clone().or_else(|| {
        std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().to_string())
    });
    let log_metadata = crate::session_log::SessionLogMetadata {
        provider: provider.clone(),
        wrapper_session_id: review_session_id,
        provider_session_id: None,
        workspace_path,
        command: "review".to_string(),
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
    let log_prompt_summary = format!(
        "review uncommitted={} base={:?} commit={:?} title={:?}",
        uncommitted, base, commit, title
    );
    crate::session_log::record_prompt(log_coordinator.writer(), Some(&log_prompt_summary))?;

    // Register process entry
    let review_proc_id = uuid::Uuid::new_v4().to_string();
    if let Ok(mut pstore) = zag_agent::process_store::ProcessStore::load() {
        pstore.add(zag_agent::process_store::ProcessEntry {
            id: review_proc_id.clone(),
            pid: std::process::id(),
            session_id: None,
            provider: provider.clone(),
            model: model_name.clone(),
            command: "review".to_string(),
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

    let review_result = agent.run(Some(&review_prompt)).await;
    match review_result {
        Ok(_) => {
            log_coordinator.finish(true, None).await?;
        }
        Err(err) => {
            if let Ok(mut pstore) = zag_agent::process_store::ProcessStore::load() {
                pstore.update_status(&review_proc_id, "killed", Some(1));
                let _ = pstore.save();
            }
            log_coordinator.finish(false, Some(err.to_string())).await?;
            return Err(err);
        }
    }

    if let Ok(mut pstore) = zag_agent::process_store::ProcessStore::load() {
        pstore.update_status(&review_proc_id, "exited", Some(0));
        let _ = pstore.save();
    }

    Ok(())
}

/// Codex-native review path using the `codex review` CLI command.
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
        quiet,
        ..
    } = params;

    debug!(
        "Starting code review via Codex (uncommitted={}, base={:?}, commit={:?})",
        uncommitted, base, commit
    );

    let spinner = logging::spinner("Initializing Codex for review".to_string());
    let mut agent = AgentFactory::create(
        "codex",
        system_prompt,
        model,
        root.clone(),
        auto_approve,
        add_dirs,
    )?;
    logging::finish_spinner_quiet(&spinner);

    let model_name = agent.get_model().to_string();
    if !quiet {
        println!(
            "\x1b[32m✓\x1b[0m Review initialized with model {}",
            model_name
        );
    }

    // Downcast to Codex to call review
    let codex = agent
        .as_any_mut()
        .downcast_mut::<crate::codex::Codex>()
        .expect("Failed to get Codex agent for review");

    let review_session_id = uuid::Uuid::new_v4().to_string();
    let workspace_path = root.clone().or_else(|| {
        std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().to_string())
    });
    let log_metadata = crate::session_log::SessionLogMetadata {
        provider: "codex".to_string(),
        wrapper_session_id: review_session_id,
        provider_session_id: None,
        workspace_path,
        command: "review".to_string(),
        model: Some(model_name.clone()),
        resumed: false,
        backfilled: false,
    };
    let live_adapter = crate::session_log::live_adapter_for_provider(
        "codex",
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
    let review_prompt = format!(
        "review uncommitted={} base={:?} commit={:?} title={:?}",
        uncommitted, base, commit, title
    );
    crate::session_log::record_prompt(log_coordinator.writer(), Some(&review_prompt))?;

    // Register process entry before execution.
    let review_proc_id = uuid::Uuid::new_v4().to_string();
    if let Ok(mut pstore) = zag_agent::process_store::ProcessStore::load() {
        pstore.add(zag_agent::process_store::ProcessEntry {
            id: review_proc_id.clone(),
            pid: std::process::id(),
            session_id: None,
            provider: "codex".to_string(),
            model: model_name.clone(),
            command: "review".to_string(),
            prompt: Some(review_prompt.chars().take(100).collect()),
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

    let review_result = codex
        .review(
            uncommitted,
            base.as_deref(),
            commit.as_deref(),
            title.as_deref(),
        )
        .await;
    match review_result {
        Ok(()) => {
            log_coordinator.finish(true, None).await?;
            Ok(())
        }
        Err(err) => {
            if let Ok(mut pstore) = zag_agent::process_store::ProcessStore::load() {
                pstore.update_status(&review_proc_id, "killed", Some(1));
                let _ = pstore.save();
            }
            log_coordinator.finish(false, Some(err.to_string())).await?;
            Err(err)
        }
    }?;

    if let Ok(mut pstore) = zag_agent::process_store::ProcessStore::load() {
        pstore.update_status(&review_proc_id, "exited", Some(0));
        let _ = pstore.save();
    }

    Ok(())
}
