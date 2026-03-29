use anyhow::{Result, bail};
use log::{debug, info};
use zag::config::Config;
use zag::factory::AgentFactory;
use zag::{auto_selector, mcp, sandbox, session, skills, worktree};

use crate::cleanup::{
    print_resume_hint, print_session_resume_hint, prompt_sandbox_cleanup, prompt_worktree_cleanup,
};
use crate::cli::Commands;
use crate::json_mode::{augment_system_prompt_for_json, handle_json_output, wrap_prompt_for_json};
use crate::resume::{
    current_workspace, discover_provider_session_id, resolve_continue_target, resolve_resume_target,
};

pub(crate) struct AgentActionParams {
    pub(crate) agent_name: String,
    pub(crate) provider: String,
    pub(crate) provider_explicit: bool,
    pub(crate) action: Commands,
    pub(crate) system_prompt: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) root: Option<String>,
    pub(crate) auto_approve: bool,
    pub(crate) add_dirs: Vec<String>,
    pub(crate) show_usage: bool,
    pub(crate) quiet: bool,
    pub(crate) verbose: bool,
    pub(crate) worktree: Option<Option<String>>,
    pub(crate) sandbox: Option<Option<String>>,
    pub(crate) size: Option<String>,
    pub(crate) json_mode: bool,
    pub(crate) json_schema: Option<serde_json::Value>,
    pub(crate) json_stream: bool,
    pub(crate) session: Option<String>,
}

pub(crate) fn run_resume_id(action: &Commands) -> Option<&str> {
    match action {
        Commands::Run { resume, .. } => resume.as_deref(),
        _ => None,
    }
}

fn run_continue_requested(action: &Commands) -> bool {
    matches!(
        action,
        Commands::Run {
            continue_session: true,
            ..
        }
    )
}

pub(crate) fn is_resume_run(action: &Commands) -> bool {
    run_resume_id(action).is_some() || run_continue_requested(action)
}

fn run_prompt(action: &Commands) -> Option<&str> {
    match action {
        Commands::Run { prompt, .. } => prompt.as_deref(),
        Commands::Exec { prompt, .. } => Some(prompt.as_str()),
        _ => None,
    }
}

fn is_new_interactive_run(action: &Commands, json_mode: bool) -> bool {
    matches!(action, Commands::Run { .. })
        && !is_resume_run(action)
        && !(json_mode && run_prompt(action).is_some())
}

/// Handle auto provider/model selection, mutating params in place.
async fn resolve_auto_selection(params: &mut AgentActionParams) -> Result<()> {
    let is_auto_provider = params.provider == "auto";
    let is_auto_model = params.model.as_deref() == Some("auto");

    if !is_auto_provider && !is_auto_model {
        return Ok(());
    }

    let task_prompt = run_prompt(&params.action);

    let task_prompt = task_prompt
        .ok_or_else(|| anyhow::anyhow!("auto provider/model requires a prompt to analyze"))?;

    let config = Config::load(params.root.as_deref()).unwrap_or_default();
    let current_provider = if !is_auto_provider {
        Some(params.provider.as_str())
    } else {
        None
    };

    let result = auto_selector::resolve(
        task_prompt,
        is_auto_provider,
        is_auto_model,
        current_provider,
        &config,
        params.root.as_deref(),
    )
    .await?;

    if let Some(p) = result.provider {
        params.provider = p;
    }
    if let Some(m) = result.model {
        params.model = Some(m);
    } else if is_auto_provider {
        params.model = None;
    }

    params.agent_name = crate::capitalize(&params.provider);

    let is_exec_action = matches!(params.action, Commands::Exec { .. });
    let show_wrapper = !params.quiet && (!is_exec_action || params.verbose);
    if show_wrapper {
        let model_info = params
            .model
            .as_deref()
            .map(|m| format!(" with model {}", m))
            .unwrap_or_default();
        println!(
            "\x1b[32m✓\x1b[0m Auto-selected: {}{}",
            params.agent_name, model_info
        );
    }

    Ok(())
}

/// Worktree setup state computed before agent creation.
pub(crate) struct WorktreeSetup {
    pub(crate) is_worktree_session: bool,
    pub(crate) session_id: Option<String>,
    pub(crate) worktree_name: Option<String>,
    pub(crate) effective_root: Option<String>,
    pub(crate) worktree_path: Option<String>,
}

pub(crate) struct PlainSessionSetup {
    pub(crate) session_id: Option<String>,
    pub(crate) workspace_path: Option<String>,
}

/// Set up worktree session state: generate IDs, create worktree.
/// All providers get the same treatment — worktree at `~/.zag/worktrees/<project>/<name>`.
fn setup_worktree(
    worktree_flag: &Option<Option<String>>,
    action: &Commands,
    root: &Option<String>,
    show_wrapper: bool,
    session_id: Option<String>,
) -> Result<WorktreeSetup> {
    let is_worktree_session = worktree_flag.is_some() && !is_resume_run(action);

    if !is_worktree_session {
        return Ok(WorktreeSetup {
            is_worktree_session: false,
            session_id: None,
            worktree_name: None,
            effective_root: root.clone(),
            worktree_path: None,
        });
    }

    let worktree_name = Some(
        worktree_flag
            .as_ref()
            .unwrap()
            .as_deref()
            .map(String::from)
            .unwrap_or_else(worktree::generate_name),
    );

    let repo_root = worktree::git_repo_root(root.as_deref())?;
    let name = worktree_name.as_deref().unwrap();
    let wt_path = worktree::create_worktree(&repo_root, name)?;
    if show_wrapper {
        println!("\x1b[32m✓\x1b[0m Worktree created at {}", wt_path.display());
    }
    let path_str = wt_path.to_string_lossy().to_string();

    Ok(WorktreeSetup {
        is_worktree_session: true,
        session_id,
        worktree_name,
        effective_root: Some(path_str.clone()),
        worktree_path: Some(path_str),
    })
}

/// Sandbox setup state computed before agent creation.
pub(crate) struct SandboxSetup {
    pub(crate) is_sandbox_session: bool,
    pub(crate) sandbox_name: Option<String>,
    pub(crate) session_id: Option<String>,
    pub(crate) workspace: Option<String>,
}

/// Set up sandbox session state: generate name, session ID, determine workspace.
fn setup_sandbox(
    sandbox_flag: &Option<Option<String>>,
    action: &Commands,
    root: &Option<String>,
    session_id: Option<String>,
) -> Result<SandboxSetup> {
    let is_sandbox_session = sandbox_flag.is_some() && !is_resume_run(action);

    if !is_sandbox_session {
        return Ok(SandboxSetup {
            is_sandbox_session: false,
            sandbox_name: None,
            session_id: None,
            workspace: None,
        });
    }

    let sandbox_name = Some(
        sandbox_flag
            .as_ref()
            .unwrap()
            .as_deref()
            .map(String::from)
            .unwrap_or_else(sandbox::generate_name),
    );

    // Determine workspace: root flag > git repo root > current dir
    let workspace = current_workspace(root.as_deref());

    Ok(SandboxSetup {
        is_sandbox_session: true,
        sandbox_name,
        session_id,
        workspace: Some(workspace),
    })
}

fn setup_plain_session(
    action: &Commands,
    json_mode: bool,
    root: &Option<String>,
    explicit_session: &Option<String>,
) -> PlainSessionSetup {
    // If an explicit --session was provided, always use it
    if let Some(session_id) = explicit_session {
        return PlainSessionSetup {
            session_id: Some(session_id.clone()),
            workspace_path: Some(current_workspace(root.as_deref())),
        };
    }

    if !is_new_interactive_run(action, json_mode) {
        return PlainSessionSetup {
            session_id: None,
            workspace_path: None,
        };
    }

    PlainSessionSetup {
        session_id: Some(uuid::Uuid::new_v4().to_string()),
        workspace_path: Some(current_workspace(root.as_deref())),
    }
}

/// Parameters for creating and configuring an agent.
struct AgentSetupParams {
    provider: String,
    agent_name: String,
    system_prompt: Option<String>,
    model: Option<String>,
    effective_root: Option<String>,
    session_id: Option<String>,
    auto_approve: bool,
    add_dirs: Vec<String>,
    output_format: Option<String>,
    input_format: Option<String>,
    replay_user_messages: bool,
    include_partial_messages: bool,
    verbose: bool,
    json_mode: bool,
    json_stream: bool,
}

/// Create and configure the agent with all settings.
fn create_and_configure_agent(
    p: AgentSetupParams,
    json_schema: &Option<serde_json::Value>,
    show_wrapper: bool,
) -> Result<(Box<dyn crate::agent::Agent + Send + Sync>, Option<String>)> {
    let spinner = if show_wrapper {
        crate::logging::spinner(format!("Initializing {} agent", p.agent_name))
    } else {
        let pb = indicatif::ProgressBar::new_spinner();
        pb.set_draw_target(indicatif::ProgressDrawTarget::hidden());
        pb
    };

    let mut agent = AgentFactory::create(
        &p.provider,
        p.system_prompt,
        p.model,
        p.effective_root,
        p.auto_approve,
        p.add_dirs,
    )?;

    let output_fmt_clone = p.output_format.clone();
    agent.set_output_format(p.output_format);

    // Configure Claude-specific options in a single downcast
    if p.provider == "claude"
        && let Some(claude_agent) = agent.as_any_mut().downcast_mut::<crate::claude::Claude>()
    {
        claude_agent.set_verbose(p.verbose);
        if let Some(session_id) = p.session_id {
            claude_agent.set_session_id(session_id);
        }
        if let Some(input_fmt) = p.input_format {
            claude_agent.set_input_format(Some(input_fmt));
        }
        if p.replay_user_messages {
            claude_agent.set_replay_user_messages(true);
        }
        if p.include_partial_messages {
            claude_agent.set_include_partial_messages(true);
        }
        if p.json_mode
            && let Some(schema) = json_schema
        {
            let schema_str = serde_json::to_string(schema).unwrap_or_default();
            claude_agent.set_json_schema(Some(schema_str));
        }

        // Set up event handler for streaming output (text or stream-json modes)
        let is_stream_json = p.json_stream || output_fmt_clone.as_deref() == Some("stream-json");
        claude_agent.set_event_handler(Box::new(move |event, verbose| {
            use crate::output::{ContentBlock, Event};
            if is_stream_json {
                // Output as unified NDJSON
                if let Ok(json) = serde_json::to_string(event) {
                    println!("{}", json);
                }
            } else {
                match event {
                    Event::Result { .. } => {
                        // End of stream — flush
                        if !verbose {
                            use std::io::Write;
                            println!();
                            let _ = std::io::stdout().flush();
                        }
                    }
                    _ => {
                        if verbose {
                            if let Some(formatted) = crate::output::format_event_as_text(event) {
                                println!("{}", formatted);
                            }
                        } else if let Event::AssistantMessage { content, .. } = event {
                            for block in content {
                                if let ContentBlock::Text { text } = block {
                                    print!("{}", text);
                                }
                            }
                        }
                    }
                }
            }
        }));
    }

    // Force output capture when JSON mode is active
    let user_output_format = output_fmt_clone.clone();
    if p.json_mode && user_output_format.is_none() {
        agent.set_output_format(Some("json".to_string()));
        if p.provider != "claude" {
            agent.set_capture_output(true);
        }
    }

    // --json-stream: set output format to stream-json (unless user already specified -o)
    if p.json_stream && user_output_format.is_none() {
        agent.set_output_format(Some("stream-json".to_string()));
    }

    crate::logging::finish_spinner_quiet(&spinner);
    debug!("Agent configuration complete");

    Ok((agent, output_fmt_clone))
}

/// Save the session-worktree/sandbox mapping to disk.
fn save_session_mapping(
    plain: &PlainSessionSetup,
    wt: &WorktreeSetup,
    sb: &SandboxSetup,
    provider: &str,
    model: &str,
    root: Option<&str>,
) {
    if plain.session_id.is_some() && !wt.is_worktree_session && !sb.is_sandbox_session {
        let mut store = session::SessionStore::load(root).unwrap_or_default();
        store.add(session::SessionEntry {
            session_id: plain.session_id.clone().unwrap_or_default(),
            provider: provider.to_string(),
            model: model.to_string(),
            worktree_path: plain.workspace_path.clone().unwrap_or_default(),
            worktree_name: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            provider_session_id: None,
            sandbox_name: None,
            is_worktree: false,
            discovered: false,
            discovery_source: None,
            log_path: None,
            log_completeness: "partial".to_string(),
        });
        if let Err(e) = store.save(root) {
            log::warn!("Failed to save session mapping: {}", e);
        }
        debug!(
            "Saved plain session mapping: id={}, model='{}'",
            plain.session_id.as_deref().unwrap_or(""),
            model
        );
    }

    // Save worktree session mapping
    if let (Some(sid), Some(wt_path), Some(wt_name)) =
        (&wt.session_id, &wt.worktree_path, &wt.worktree_name)
    {
        let mut store = session::SessionStore::load(root).unwrap_or_default();
        store.add(session::SessionEntry {
            session_id: sid.clone(),
            provider: provider.to_string(),
            model: model.to_string(),
            worktree_path: wt_path.clone(),
            worktree_name: wt_name.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            provider_session_id: None,
            sandbox_name: None,
            is_worktree: true,
            discovered: false,
            discovery_source: None,
            log_path: None,
            log_completeness: "partial".to_string(),
        });
        if let Err(e) = store.save(root) {
            log::warn!("Failed to save session mapping: {}", e);
        }
        debug!("Saved session mapping: {} -> {}", sid, wt_path);
    }

    // Save sandbox session mapping
    if let (Some(sid), Some(sandbox_name)) = (&sb.session_id, &sb.sandbox_name) {
        let workspace = sb.workspace.clone().unwrap_or_default();
        let mut store = session::SessionStore::load(root).unwrap_or_default();
        store.add(session::SessionEntry {
            session_id: sid.clone(),
            provider: provider.to_string(),
            model: model.to_string(),
            worktree_path: workspace.clone(),
            worktree_name: sandbox_name.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            provider_session_id: None,
            sandbox_name: Some(sandbox_name.clone()),
            is_worktree: false,
            discovered: false,
            discovery_source: None,
            log_path: None,
            log_completeness: "partial".to_string(),
        });
        if let Err(e) = store.save(root) {
            log::warn!("Failed to save sandbox session mapping: {}", e);
        }
        debug!("Saved sandbox session mapping: {} -> {}", sid, sandbox_name);
    }
}

fn update_provider_session_id(
    wrapper_session_id: Option<&str>,
    provider_session_id: Option<String>,
    root: Option<&str>,
) {
    let (Some(wrapper_session_id), Some(provider_session_id)) =
        (wrapper_session_id, provider_session_id)
    else {
        return;
    };

    let mut store = session::SessionStore::load(root).unwrap_or_default();
    store.set_provider_session_id(wrapper_session_id, provider_session_id);
    if let Err(e) = store.save(root) {
        log::warn!("Failed to update provider session id: {}", e);
    }
}

/// Context for executing an action.
struct ExecutionContext<'a> {
    provider: &'a str,
    json_mode: bool,
    json_schema: &'a Option<serde_json::Value>,
    output_fmt: Option<&'a str>,
    show_usage: bool,
    verbose: bool,
}

/// Execute the requested action.
async fn execute_action(
    action: Commands,
    agent: &mut (dyn crate::agent::Agent + Send + Sync),
    ctx: &ExecutionContext<'_>,
    log_writer: Option<&crate::session_log::SessionLogWriter>,
) -> Result<()> {
    match action {
        Commands::Run {
            prompt,
            resume,
            continue_session,
            ..
        } => {
            if resume.is_some() || continue_session {
                if let Some(ref session_id) = resume {
                    info!("Resuming session {}", session_id);
                } else {
                    info!("Resuming latest session");
                }

                agent
                    .run_resume(resume.as_deref(), continue_session)
                    .await?;
            } else if ctx.json_mode && prompt.is_some() {
                info!("Starting non-interactive session (JSON mode)");
                let wrapped = if ctx.provider != "claude" {
                    let w = prompt.as_deref().map(wrap_prompt_for_json);
                    if let Some(ref wp) = w {
                        debug!("JSON-wrapped run prompt: {}", wp);
                    }
                    w
                } else {
                    debug!("Run prompt (JSON mode, Claude): {:?}", prompt);
                    None
                };
                let run_prompt = wrapped.as_deref().or(prompt.as_deref());
                let agent_output = agent.run(run_prompt).await?;
                if let (Some(writer), Some(agent_output)) = (log_writer, agent_output.as_ref()) {
                    crate::session_log::record_agent_output(writer, agent_output)?;
                }
                handle_json_output(
                    agent_output,
                    agent,
                    ctx.json_schema,
                    ctx.show_usage,
                    ctx.verbose,
                )
                .await?;
            } else {
                info!("Starting interactive session");
                agent.run_interactive(prompt.as_deref()).await?;
            }
        }
        Commands::Exec { prompt, .. } => {
            info!("Starting non-interactive session");
            let run_prompt = if ctx.json_mode && ctx.provider != "claude" {
                let wrapped = wrap_prompt_for_json(&prompt);
                debug!("JSON-wrapped prompt: {}", wrapped);
                wrapped
            } else {
                debug!("Exec prompt: {}", prompt);
                prompt.clone()
            };
            let agent_output = agent.run(Some(&run_prompt)).await?;
            if let (Some(writer), Some(agent_output)) = (log_writer, agent_output.as_ref()) {
                crate::session_log::record_agent_output(writer, agent_output)?;
            }

            if ctx.json_mode {
                handle_json_output(
                    agent_output,
                    agent,
                    ctx.json_schema,
                    ctx.show_usage,
                    ctx.verbose,
                )
                .await?;
            } else if let Some(agent_out) = agent_output {
                print_agent_output(&agent_out, ctx.output_fmt, ctx.show_usage, ctx.verbose)?;
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}

/// Print agent output in the requested format.
fn print_agent_output(
    agent_out: &crate::output::AgentOutput,
    output_fmt: Option<&str>,
    show_usage: bool,
    verbose: bool,
) -> Result<()> {
    match output_fmt {
        Some("json") => {
            let json = serde_json::to_string(agent_out)?;
            println!("{}", json);
        }
        Some("json-pretty") => {
            let json = serde_json::to_string_pretty(agent_out)?;
            println!("{}", json);
        }
        Some("stream-json") => {
            for event in &agent_out.events {
                let json = serde_json::to_string(event)?;
                println!("{}", json);
            }
        }
        _ => {
            process_agent_output(agent_out, show_usage, verbose)?;
        }
    }
    Ok(())
}

/// Log configuration details at debug level.
fn log_config_details(params: &AgentActionParams) {
    if let Some(ref m) = params.model {
        debug!("Model specified: {}", m);
    }
    if let Some(ref r) = params.root {
        debug!("Root directory: {}", r);
    }
    if params.auto_approve {
        debug!("Auto-approve enabled");
    }
    if let Some(ref sp) = params.system_prompt {
        debug!("System prompt: {}", sp);
    }
    if !params.add_dirs.is_empty() {
        debug!("Additional directories: {:?}", params.add_dirs);
    }
    if params.worktree.is_some() {
        debug!("Worktree mode enabled");
    }
    if params.sandbox.is_some() {
        debug!("Sandbox mode enabled");
    }
    if params.json_mode {
        debug!("JSON output mode enabled");
    }
}

fn command_name(action: &Commands) -> &'static str {
    match action {
        Commands::Run { .. } => "run",
        Commands::Exec { .. } => "exec",
        Commands::Review { .. } => "review",
        Commands::Config { .. } => "config",
        Commands::Session { .. } => "session",
        Commands::Capability { .. } => "capability",
        Commands::Listen { .. } => "listen",
        Commands::Man { .. } => "man",
        Commands::Skills { .. } => "skills",
        Commands::Mcp { .. } => "mcp",
        Commands::Ps { .. } => "ps",
        Commands::Search { .. } => "search",
        Commands::Input { .. } => "input",
    }
}

fn action_prompt(action: &Commands) -> Option<&str> {
    match action {
        Commands::Run { prompt, .. } => prompt.as_deref(),
        Commands::Exec { prompt, .. } => Some(prompt.as_str()),
        _ => None,
    }
}

fn should_enable_live_session_logs(action: &Commands, json_mode: bool) -> bool {
    matches!(action, Commands::Run { .. }) && !json_mode
}

fn update_session_log_metadata(
    session_id: Option<&str>,
    log_path: Option<String>,
    completeness: &str,
    root: Option<&str>,
) {
    let Some(session_id) = session_id else {
        return;
    };
    let mut store = session::SessionStore::load(root).unwrap_or_default();
    if let Some(entry) = store
        .sessions
        .iter_mut()
        .find(|entry| entry.session_id == session_id)
    {
        entry.log_path = log_path;
        entry.log_completeness = completeness.to_string();
        let _ = store.save(root);
    }
}

pub(crate) async fn run_agent_action(mut params: AgentActionParams) -> Result<()> {
    resolve_auto_selection(&mut params).await?;
    log_config_details(&params);

    let AgentActionParams {
        agent_name: _,
        mut provider,
        provider_explicit,
        mut action,
        system_prompt,
        mut model,
        root,
        auto_approve,
        add_dirs,
        show_usage,
        quiet,
        verbose,
        worktree: worktree_flag,
        sandbox: sandbox_flag,
        size,
        json_mode,
        json_schema,
        json_stream,
        session,
    } = params;

    let is_exec = matches!(action, Commands::Exec { .. });
    let show_wrapper = !quiet && (!is_exec || verbose);

    let mut system_prompt =
        augment_system_prompt_for_json(system_prompt, json_mode, &provider, &json_schema);

    if let Err(e) = skills::setup_skills(&provider, &mut system_prompt) {
        log::warn!("Failed to set up skills: {}", e);
    }

    if let Err(e) = mcp::setup_mcp(&provider, root.as_deref()) {
        log::warn!("Failed to set up MCP servers: {}", e);
    }

    if let Some(ref sp) = system_prompt {
        debug!("Effective system prompt: {}", sp);
    }

    let resume_target = if let Some(session_id) = run_resume_id(&action) {
        resolve_resume_target(session_id, root.as_deref())
    } else if run_continue_requested(&action) {
        resolve_continue_target(root.as_deref())
    } else {
        None
    };

    if is_resume_run(&action) && resume_target.is_none() {
        bail!("No matching session found to resume");
    }

    if let Some(target) = &resume_target {
        if provider_explicit && provider != target.entry.provider {
            bail!(
                "Requested provider '{}' does not match the stored session provider '{}'",
                provider,
                target.entry.provider
            );
        }
        provider = target.entry.provider.clone();
        if !target.entry.model.is_empty() {
            debug!(
                "Restored model from session entry: '{}'",
                target.entry.model
            );
            model = Some(target.entry.model.clone());
        } else {
            debug!("Session entry has empty model, will fall back to config/default");
        }
    }

    if let Some(target) = &resume_target {
        let native_id = target
            .entry
            .provider_session_id
            .clone()
            .unwrap_or_else(|| target.entry.session_id.clone());
        if let Commands::Run {
            resume,
            continue_session,
            ..
        } = &mut action
        {
            *resume = Some(native_id);
            *continue_session = false;
        }
    }

    let plain = setup_plain_session(&action, json_mode, &root, &session);
    let wrapper_session_id = plain.session_id.clone();
    let log_session_id = wrapper_session_id
        .clone()
        .or_else(|| {
            resume_target
                .as_ref()
                .map(|target| target.entry.session_id.clone())
        })
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let wt = setup_worktree(
        &worktree_flag,
        &action,
        &root,
        show_wrapper,
        wrapper_session_id.clone(),
    )?;
    let sb = setup_sandbox(&sandbox_flag, &action, &root, wrapper_session_id.clone())?;

    let effective_root = if let Some(target) = &resume_target {
        if target.entry.is_worktree {
            let wt_path = std::path::Path::new(&target.entry.worktree_path);
            if !wt_path.exists() && target.matched_by_wrapper_id {
                log::warn!(
                    "Worktree no longer exists at {}, resuming without it",
                    target.entry.worktree_path
                );
                let mut store = session::SessionStore::load(root.as_deref()).unwrap_or_default();
                store.remove(&target.entry.session_id);
                let _ = store.save(root.as_deref());
                Some(current_workspace(root.as_deref()))
            } else {
                Some(target.entry.worktree_path.clone())
            }
        } else {
            Some(target.entry.worktree_path.clone())
        }
    } else {
        wt.effective_root
            .clone()
            .or_else(|| plain.workspace_path.clone())
    };

    // Extract output/input format and streaming flags from exec action
    let (output_format, input_format, replay_user_messages, include_partial_messages) =
        match &action {
            Commands::Exec {
                output,
                input_format,
                replay_user_messages,
                include_partial_messages,
                ..
            } => (
                output.clone(),
                input_format.clone(),
                *replay_user_messages,
                *include_partial_messages,
            ),
            _ => (None, None, false, false),
        };

    if let Some(ref o) = output_format {
        debug!("Output format: {}", o);
    }
    if let Some(ref i) = input_format {
        debug!("Input format: {}", i);
    }

    let (mut agent, output_fmt_clone) = create_and_configure_agent(
        AgentSetupParams {
            provider: provider.clone(),
            agent_name: crate::capitalize(&provider),
            system_prompt,
            model,
            effective_root: effective_root.clone(),
            session_id: wrapper_session_id.clone(),
            auto_approve,
            add_dirs,
            output_format,
            input_format,
            replay_user_messages,
            include_partial_messages,
            verbose,
            json_mode,
            json_stream,
        },
        &json_schema,
        show_wrapper,
    )?;

    // Configure sandbox if active
    if sb.is_sandbox_session
        && let (Some(name), Some(workspace)) = (&sb.sandbox_name, &sb.workspace)
    {
        let config = sandbox::SandboxConfig {
            name: name.clone(),
            template: sandbox::template_for_provider(&provider).to_string(),
            workspace: workspace.clone(),
        };
        agent.set_sandbox(config);
        if show_wrapper {
            println!("\x1b[32m✓\x1b[0m Sandbox configured: {}", name);
        }
    }
    if let Some(target) = &resume_target
        && let Some(name) = &target.entry.sandbox_name
    {
        let config = sandbox::SandboxConfig {
            name: name.clone(),
            template: sandbox::template_for_provider(&provider).to_string(),
            workspace: target.entry.worktree_path.clone(),
        };
        agent.set_sandbox(config);
        if show_wrapper {
            println!("\x1b[32m✓\x1b[0m Sandbox configured: {}", name);
        }
    }

    // Configure Ollama-specific options (model + size from config, --size flag)
    if provider == "ollama" {
        let config = Config::load(root.as_deref()).unwrap_or_default();

        // If --model was a size alias (small/medium/large), the factory resolved it
        // to a size string (e.g., "2b") via model_for_size — treat that as a --size instead.
        let current_model = agent.get_model().to_string();
        let is_size_value = crate::ollama::AVAILABLE_SIZES.contains(&current_model.as_str());
        if is_size_value {
            // --model was a size alias — revert model to config default, use resolved value as size
            agent.set_model(config.ollama_model().to_string());
        } else if current_model == crate::ollama::DEFAULT_MODEL {
            // No --model flag (or it matched default) — use config model
            agent.set_model(config.ollama_model().to_string());
        }
        // else: --model was an explicit model name — keep it

        if let Some(ollama_agent) = agent.as_any_mut().downcast_mut::<crate::ollama::Ollama>() {
            // Resolve size: --size flag > size-from-alias > ollama.size config > default
            if let Some(ref s) = size {
                let resolved = config.ollama_size_for(s).to_string();
                ollama_agent.set_size(resolved);
            } else if is_size_value {
                ollama_agent.set_size(current_model);
            } else {
                ollama_agent.set_size(config.ollama_size().to_string());
            }
        }
    }

    // Display initialization message
    let model_display = if provider == "ollama" {
        // Show full model:size tag for ollama
        if let Some(ollama_agent) = agent.as_any_mut().downcast_mut::<crate::ollama::Ollama>() {
            ollama_agent.display_model()
        } else {
            agent.get_model().to_string()
        }
    } else {
        agent.get_model().to_string()
    };
    let persisted_model = agent.get_model().to_string();
    let auto_approve_suffix = if auto_approve { " (auto approve)" } else { "" };
    if show_wrapper {
        println!(
            "\x1b[32m✓\x1b[0m {} initialized with model {}{}",
            crate::capitalize(&provider),
            model_display,
            auto_approve_suffix
        );
    }

    // Save session-worktree mapping before execution (so it survives Ctrl+C)
    save_session_mapping(
        &plain,
        &wt,
        &sb,
        &provider,
        &persisted_model,
        root.as_deref(),
    );

    // Register process entry before execution so `zag ps` can see it while running.
    let proc_id = uuid::Uuid::new_v4().to_string();
    let proc_session_id = wt
        .session_id
        .clone()
        .or_else(|| sb.session_id.clone())
        .or_else(|| plain.session_id.clone());
    let proc_prompt = action_prompt(&action).map(|p| p.chars().take(100).collect::<String>());
    let proc_cmd = command_name(&action).to_string();
    if let Ok(mut pstore) = zag::process_store::ProcessStore::load() {
        pstore.add(zag::process_store::ProcessEntry {
            id: proc_id.clone(),
            pid: std::process::id(),
            session_id: proc_session_id,
            provider: provider.clone(),
            model: persisted_model.clone(),
            command: proc_cmd,
            prompt: proc_prompt,
            started_at: chrono::Utc::now().to_rfc3339(),
            status: "running".to_string(),
            exit_code: None,
            exited_at: None,
            root: root.clone(),
        });
        let _ = pstore.save();
    }

    // Echo session ID for `agent listen` usage
    if show_wrapper {
        let display_session_id = wt
            .session_id
            .as_deref()
            .or(sb.session_id.as_deref())
            .or(plain.session_id.as_deref())
            .unwrap_or(&log_session_id);
        println!("\x1b[33m>\x1b[0m Session: {}", display_session_id);
        println!(
            "\x1b[33m>\x1b[0m Listen:  agent listen {}",
            display_session_id
        );
    }

    let initial_provider_session_id = if provider == "claude" {
        wrapper_session_id.clone()
    } else {
        resume_target
            .as_ref()
            .and_then(|target| target.entry.provider_session_id.clone())
    };
    let log_metadata = crate::session_log::SessionLogMetadata {
        provider: provider.clone(),
        wrapper_session_id: log_session_id.clone(),
        provider_session_id: initial_provider_session_id,
        workspace_path: effective_root
            .clone()
            .or_else(|| plain.workspace_path.clone())
            .or_else(|| wt.worktree_path.clone())
            .or_else(|| sb.workspace.clone()),
        command: command_name(&action).to_string(),
        model: Some(persisted_model.clone()),
        resumed: is_resume_run(&action),
        backfilled: false,
    };
    let live_ctx = crate::session_log::LiveLogContext {
        root: root.clone(),
        provider_session_id: log_metadata.provider_session_id.clone(),
        workspace_path: log_metadata.workspace_path.clone(),
        started_at: chrono::Utc::now(),
        is_worktree: wt.is_worktree_session,
    };
    let live_adapter = crate::session_log::live_adapter_for_provider(
        &provider,
        live_ctx,
        should_enable_live_session_logs(&action, json_mode),
    );
    let log_coordinator = crate::session_log::SessionLogCoordinator::start(
        &crate::session_log::logs_dir(root.as_deref()),
        log_metadata,
        live_adapter,
    )?;
    let _ = log_coordinator
        .writer()
        .set_global_index_dir(Config::global_base_dir());
    crate::session_log::record_prompt(log_coordinator.writer(), action_prompt(&action))?;
    if let Ok(log_path) = log_coordinator.writer().log_path() {
        update_session_log_metadata(
            wrapper_session_id
                .as_deref()
                .or(wt.session_id.as_deref())
                .or(sb.session_id.as_deref()),
            Some(log_path.to_string_lossy().to_string()),
            "partial",
            root.as_deref(),
        );
    }

    let is_worktree_session = wt.is_worktree_session;
    let is_interactive_worktree = wt.is_worktree_session && matches!(action, Commands::Run { .. });
    let is_interactive_sandbox = sb.is_sandbox_session && matches!(action, Commands::Run { .. });
    let is_interactive_run = matches!(action, Commands::Run { .. });

    let exec_ctx = ExecutionContext {
        provider: &provider,
        json_mode,
        json_schema: &json_schema,
        output_fmt: output_fmt_clone.as_deref(),
        show_usage,
        verbose,
    };
    let action_result = execute_action(
        action,
        &mut *agent,
        &exec_ctx,
        Some(log_coordinator.writer()),
    )
    .await;
    if let Err(err) = &action_result {
        if let Ok(mut pstore) = zag::process_store::ProcessStore::load() {
            pstore.update_status(&proc_id, "killed", Some(1));
            let _ = pstore.save();
        }
        log_coordinator.finish(false, Some(err.to_string())).await?;
        return Err(anyhow::anyhow!(err.to_string()));
    }

    let wrapper_session_id = wt
        .session_id
        .as_deref()
        .or(sb.session_id.as_deref())
        .or(plain.session_id.as_deref());
    // Prefer the provider session ID discovered by the live log adapter during the session.
    // Fall back to post-session discovery only if the live adapter didn't find one
    // (or found one identical to the wrapper UUID, which is not a real native ID).
    let live_discovered_id = log_coordinator.writer().get_provider_session_id();
    // Use the effective workspace path (worktree/sandbox path if applicable) for provider
    // session discovery, not plain.workspace_path which is always the original repo root.
    let discovery_workspace = effective_root
        .as_deref()
        .or(plain.workspace_path.as_deref());
    let native_session_id = live_discovered_id
        .filter(|id| wrapper_session_id.is_none_or(|wid| id != wid))
        .or_else(|| {
            discover_provider_session_id(
                &provider,
                wrapper_session_id,
                root.as_deref(),
                discovery_workspace,
            )
        });
    if let Some(ref native_id) = native_session_id {
        log_coordinator
            .writer()
            .set_provider_session_id(Some(native_id.clone()))?;
    }
    update_provider_session_id(wrapper_session_id, native_session_id, root.as_deref());
    update_session_log_metadata(
        wrapper_session_id,
        log_coordinator
            .writer()
            .log_path()
            .ok()
            .map(|path| path.to_string_lossy().to_string()),
        "partial",
        root.as_deref(),
    );
    log_coordinator.finish(true, None).await?;

    if let Ok(mut pstore) = zag::process_store::ProcessStore::load() {
        pstore.update_status(&proc_id, "exited", Some(0));
        let _ = pstore.save();
    }

    // Cleanup
    debug!("Cleaning up agent resources");
    agent.cleanup().await?;
    if show_wrapper {
        println!("\x1b[32m✓\x1b[0m Session terminated");
    }

    // Sandbox cleanup prompt
    if is_interactive_sandbox {
        if let Some(ref name) = sb.sandbox_name {
            prompt_sandbox_cleanup(
                sb.session_id.as_deref().unwrap_or(""),
                name,
                root.as_deref(),
            )?;
        }
    } else if let Some(target) = &resume_target
        && let Some(ref sandbox_name) = target.entry.sandbox_name
        && target.matched_by_wrapper_id
    {
        let sid = target.entry.session_id.as_str();
        prompt_sandbox_cleanup(sid, sandbox_name, root.as_deref())?;
    }

    // Worktree cleanup
    // For interactive sessions: auto-delete if no changes, prompt if changes exist
    // For exec sessions: auto-delete if no changes, keep if changes exist
    let cleanup_info = if is_worktree_session {
        wt.session_id
            .as_ref()
            .zip(wt.worktree_path.as_ref())
            .map(|(sid, wtp)| (sid.clone(), wtp.clone()))
    } else if let Some(target) = &resume_target {
        if target.entry.is_worktree && target.matched_by_wrapper_id {
            Some((
                target.entry.session_id.clone(),
                target.entry.worktree_path.clone(),
            ))
        } else {
            None
        }
    } else {
        None
    };

    if let Some((sid, wtp)) = cleanup_info {
        let wt_path = std::path::Path::new(&wtp);
        let has_changes = wt_path.exists()
            && (worktree::has_changes(wt_path).unwrap_or(true)
                || worktree::has_unpushed_commits(wt_path).unwrap_or(true));

        if !has_changes {
            // Auto-remove worktree with no changes
            if wt_path.exists() {
                match worktree::remove_worktree(wt_path) {
                    Ok(()) => {
                        if show_wrapper {
                            println!("\x1b[32m✓\x1b[0m Worktree removed (no changes)");
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to remove worktree: {}", e);
                    }
                }
            }
            let mut store = session::SessionStore::load(root.as_deref()).unwrap_or_default();
            store.remove(&sid);
            let _ = store.save(root.as_deref());
        } else if is_interactive_worktree {
            prompt_worktree_cleanup(&sid, &wtp, root.as_deref())?;
        } else {
            // Exec with changes: keep and print resume command
            if show_wrapper {
                let store = session::SessionStore::load(root.as_deref()).unwrap_or_default();
                let provider_session_id = store
                    .find_by_session_id(&sid)
                    .and_then(|entry| entry.provider_session_id.as_deref());
                print_resume_hint(&sid, provider_session_id, "Workspace");
            }
        }
    } else if let Some(wrapper_session_id) = wrapper_session_id {
        // Plain interactive session (no worktree/sandbox): print resume hint
        if is_interactive_run && show_wrapper {
            let store = session::SessionStore::load(root.as_deref()).unwrap_or_default();
            let provider_session_id = store
                .find_by_session_id(wrapper_session_id)
                .and_then(|entry| entry.provider_session_id.clone());
            print_session_resume_hint(wrapper_session_id, provider_session_id.as_deref());
        }
    } else if is_interactive_run
        && show_wrapper
        && let Some(target) = &resume_target
        && !target.entry.is_worktree
        && target.entry.sandbox_name.is_none()
    {
        // Resumed plain session (no worktree/sandbox): print resume hint again
        let sid = &target.entry.session_id;
        let store = session::SessionStore::load(root.as_deref()).unwrap_or_default();
        let provider_session_id = store
            .find_by_session_id(sid)
            .and_then(|entry| entry.provider_session_id.clone());
        print_session_resume_hint(sid, provider_session_id.as_deref());
    }

    Ok(())
}

/// Process and display structured agent output
fn process_agent_output(
    output: &crate::output::AgentOutput,
    show_usage: bool,
    verbose: bool,
) -> Result<()> {
    use crate::output::{Event, LogLevel};

    // Show decorations only when verbose is enabled (or not in quiet mode for non-exec paths)
    let show_decorations = verbose && !crate::logging::is_quiet();

    if show_decorations {
        let min_level = LogLevel::Info;

        let log_entries = output.to_log_entries(min_level);
        for entry in log_entries {
            match entry.level {
                LogLevel::Debug => debug!("{}", entry.message),
                LogLevel::Info => info!("{}", entry.message),
                LogLevel::Warn => log::warn!("{}", entry.message),
                LogLevel::Error => log::error!("{}", entry.message),
            }
        }

        for event in &output.events {
            if let Event::ToolExecution {
                tool_name, result, ..
            } = event
            {
                if result.success {
                    info!("✓ Tool '{}' executed successfully", tool_name);
                } else {
                    log::warn!(
                        "✗ Tool '{}' failed: {}",
                        tool_name,
                        result.error.as_deref().unwrap_or("unknown error")
                    );
                }
            }
        }
    }

    // Display final result if available (always shown)
    if let Some(result) = output.final_result() {
        if show_decorations {
            println!("\n{}", result);
        } else {
            println!("{}", result);
        }
    }

    if show_decorations {
        if let Some(cost) = output.total_cost_usd {
            info!("Total cost: ${:.4}", cost);
        }

        if show_usage && let Some(usage) = &output.usage {
            info!(
                "Token usage - Input: {}, Output: {}",
                usage.input_tokens, usage.output_tokens
            );

            if let Some(cache_read) = usage.cache_read_tokens
                && cache_read > 0
            {
                info!("Cache read: {} tokens", cache_read);
            }

            if let Some(cache_creation) = usage.cache_creation_tokens
                && cache_creation > 0
            {
                info!("Cache created: {} tokens", cache_creation);
            }

            if let Some(web_search) = usage.web_search_requests
                && web_search > 0
            {
                info!("Web search requests: {}", web_search);
            }

            if let Some(web_fetch) = usage.web_fetch_requests
                && web_fetch > 0
            {
                info!("Web fetch requests: {}", web_fetch);
            }
        }
    }

    Ok(())
}
