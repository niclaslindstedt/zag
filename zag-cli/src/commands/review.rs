use anyhow::{Result, bail};
use log::debug;

use crate::config::Config;
use crate::factory::AgentFactory;
use crate::logging;
use zag_agent::review as lib_review;

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
        "Starting code review via {provider} (uncommitted={uncommitted}, base={base:?}, commit={commit:?})"
    );

    let diff = lib_review::gather_diff(
        uncommitted,
        base.as_deref(),
        commit.as_deref(),
        root.as_deref(),
    )?;
    let review_prompt = lib_review::build_review_prompt(&diff, title.as_deref(), prompt.as_deref());

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
        println!("\x1b[32m✓\x1b[0m Review initialized with model {model_name}");
    }

    let (log_coordinator, proc_id) = setup_review_bookkeeping(
        &provider,
        &model_name,
        root.as_deref(),
        format!("review uncommitted={uncommitted} base={base:?} commit={commit:?} title={title:?}"),
    )?;

    let review_result = agent.run(Some(&review_prompt)).await;
    finalize_review_bookkeeping(log_coordinator, proc_id, review_result).await?;
    Ok(())
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
        quiet,
        ..
    } = params;

    debug!(
        "Starting code review via Codex (uncommitted={uncommitted}, base={base:?}, commit={commit:?})"
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
        println!("\x1b[32m✓\x1b[0m Review initialized with model {model_name}");
    }

    let (log_coordinator, proc_id) = setup_review_bookkeeping(
        "codex",
        &model_name,
        root.as_deref(),
        format!("review uncommitted={uncommitted} base={base:?} commit={commit:?} title={title:?}"),
    )?;

    let codex = agent
        .as_any_mut()
        .downcast_mut::<crate::codex::Codex>()
        .expect("Failed to get Codex agent for review");

    let review_result: Result<()> = codex
        .review(
            uncommitted,
            base.as_deref(),
            commit.as_deref(),
            title.as_deref(),
        )
        .await;
    finalize_review_bookkeeping(log_coordinator, proc_id, review_result).await
}

fn setup_review_bookkeeping(
    provider: &str,
    model_name: &str,
    root: Option<&str>,
    prompt_summary: String,
) -> Result<(crate::session_log::SessionLogCoordinator, String)> {
    let review_session_id = uuid::Uuid::new_v4().to_string();
    let workspace_path = root.map(String::from).or_else(|| {
        std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().to_string())
    });
    let log_metadata = crate::session_log::SessionLogMetadata {
        provider: provider.to_string(),
        wrapper_session_id: review_session_id,
        provider_session_id: None,
        workspace_path,
        command: "review".to_string(),
        model: Some(model_name.to_string()),
        resumed: false,
        backfilled: false,
    };
    let live_adapter = crate::session_log::live_adapter_for_provider(
        provider,
        crate::session_log::LiveLogContext {
            root: root.map(String::from),
            provider_session_id: None,
            workspace_path: log_metadata.workspace_path.clone(),
            started_at: chrono::Utc::now(),
            is_worktree: false,
        },
        true,
    );
    let log_coordinator = crate::session_log::SessionLogCoordinator::start(
        &crate::session_log::logs_dir(root),
        log_metadata,
        live_adapter,
    )?;
    let _ = log_coordinator
        .writer()
        .set_global_index_dir(Config::global_base_dir());
    crate::session_log::record_prompt(log_coordinator.writer(), Some(&prompt_summary))?;

    let review_proc_id = uuid::Uuid::new_v4().to_string();
    if let Ok(mut pstore) = zag_agent::process_store::ProcessStore::load() {
        pstore.add(zag_agent::process_store::ProcessEntry {
            id: review_proc_id.clone(),
            pid: std::process::id(),
            session_id: None,
            provider: provider.to_string(),
            model: model_name.to_string(),
            command: "review".to_string(),
            prompt: Some(prompt_summary.chars().take(100).collect()),
            started_at: chrono::Utc::now().to_rfc3339(),
            status: "running".to_string(),
            exit_code: None,
            exited_at: None,
            root: root.map(String::from),
            parent_process_id: std::env::var("ZAG_PROCESS_ID").ok(),
            parent_session_id: std::env::var("ZAG_SESSION_ID").ok(),
        });
        let _ = pstore.save();
    }

    Ok((log_coordinator, review_proc_id))
}

async fn finalize_review_bookkeeping<T>(
    log_coordinator: crate::session_log::SessionLogCoordinator,
    proc_id: String,
    result: Result<T>,
) -> Result<()> {
    match result {
        Ok(_) => {
            log_coordinator.finish(true, None).await?;
            if let Ok(mut pstore) = zag_agent::process_store::ProcessStore::load() {
                pstore.update_status(&proc_id, "exited", Some(0));
                let _ = pstore.save();
            }
            Ok(())
        }
        Err(err) => {
            if let Ok(mut pstore) = zag_agent::process_store::ProcessStore::load() {
                pstore.update_status(&proc_id, "killed", Some(1));
                let _ = pstore.save();
            }
            log_coordinator.finish(false, Some(err.to_string())).await?;
            Err(err)
        }
    }
}
