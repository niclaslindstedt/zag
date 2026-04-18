use anyhow::{Context, Result};
use log::debug;

use crate::config::Config;
use crate::factory::AgentFactory;
use crate::logging;
use zag_agent::plan as lib_plan;

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

    let output_path = match output {
        Some(ref out) => {
            let resolved = lib_plan::resolve_output_path(out);
            lib_plan::validate_output_path(&resolved)?;
            Some(resolved)
        }
        None => None,
    };

    let plan_prompt = lib_plan::build_plan_prompt(&goal, instructions.as_deref());

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

    if output_path.is_some() {
        agent.set_capture_output(true);
    }

    if !quiet {
        eprintln!("\x1b[32m✓\x1b[0m Plan initialized with model {model_name}");
    }

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
