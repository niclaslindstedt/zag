use anyhow::{Result, bail};
use log::debug;

use crate::config::Config;
use crate::factory::AgentFactory;
use crate::logging;

pub(crate) struct ReviewParams {
    pub uncommitted: bool,
    pub base: Option<String>,
    pub commit: Option<String>,
    pub title: Option<String>,
    pub system_prompt: Option<String>,
    pub model: Option<String>,
    pub root: Option<String>,
    pub auto_approve: bool,
    pub add_dirs: Vec<String>,
    pub quiet: bool,
}

pub(crate) async fn run_review(params: ReviewParams) -> Result<()> {
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
    } = params;
    if !uncommitted && base.is_none() && commit.is_none() {
        bail!("Review requires at least one of: --uncommitted, --base <BRANCH>, --commit <SHA>");
    }

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
