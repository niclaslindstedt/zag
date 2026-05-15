use anyhow::{Context, Result, bail};
use log::{debug, info};
use zag_agent::config::Config;
use zag_agent::factory::AgentFactory;
use zag_agent::{auto_selector, mcp, sandbox, session, skills, worktree};

use crate::cleanup::{
    print_resume_hint, print_session_resume_hint, prompt_sandbox_cleanup, prompt_worktree_cleanup,
};
use crate::cli::Commands;
use crate::json_mode::{augment_system_prompt_for_json, handle_json_output, wrap_prompt_for_json};
use crate::output::print_agent_output;
use crate::resume::{
    current_workspace, discover_provider_session_id, resolve_continue_target, resolve_resume_target,
};
use crate::session_setup::{
    SessionMetadata, save_session_mapping, setup_plain_session, setup_sandbox, setup_worktree,
    update_provider_session_id, update_session_log_metadata,
};

/// Exit code: agent reported failure (is_error = true in AgentOutput).
const EXIT_AGENT_FAILURE: i32 = 1;

/// Exit code: underlying provider process crashed or exited with non-zero status.
const EXIT_PROVIDER_ERROR: i32 = 2;

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
    pub(crate) session: Option<String>,
    pub(crate) max_turns: Option<u32>,
    pub(crate) mcp_config: Option<String>,
    pub(crate) timeout: Option<String>,
    pub(crate) exit_on_failure: bool,
    pub(crate) context_session: Option<String>,
    pub(crate) plan_path: Option<String>,
    pub(crate) env_vars: Vec<(String, String)>,
    pub(crate) files: Vec<String>,
    pub(crate) session_metadata: SessionMetadata,
    /// `--exit` flag state: `None` when unset, otherwise an
    /// [`ExitHint`](zag_agent::exit_mode::ExitHint) describing
    /// whether it was passed bare or with a hint string.
    pub(crate) exit_hint: Option<zag_agent::exit_mode::ExitHint>,
    /// `--headless` flag: when true, attach the provider's interactive
    /// TUI to a private PTY so it is invisible to the operator. Validated
    /// to require `auto_approve` + `exit_hint` + Claude provider.
    pub(crate) headless: bool,
}

pub(crate) fn run_resume_id(action: &Commands) -> Option<&str> {
    match action {
        Commands::Run { resume, .. } | Commands::Exec { resume, .. } => resume.as_deref(),
        _ => None,
    }
}

fn run_continue_requested(action: &Commands) -> bool {
    matches!(
        action,
        Commands::Run {
            continue_session: true,
            ..
        } | Commands::Exec {
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

fn is_new_interactive_run(action: &Commands, json_mode: bool, exit_active: bool) -> bool {
    matches!(action, Commands::Run { .. })
        && !is_resume_run(action)
        // JSON mode + prompt normally short-circuits to a single-shot
        // run, but `--exit` always wants an interactive session (the
        // agent terminates by calling `zag ps kill self <result>`).
        && (exit_active || !(json_mode && run_prompt(action).is_some()))
}

/// Resolve the effective provider by walking the fallback tier list.
///
/// When the user has not pinned a provider with `-p`, this walks
/// `AgentFactory::fallback_sequence(requested)` and picks the first
/// provider whose binary is present in PATH and whose startup probe
/// succeeds. Each downgrade is logged via `log::warn!` so the user can
/// see which provider actually ended up running and why.
///
/// When `provider_explicit` is true, this is a no-op that just returns
/// the requested provider — explicit pinning must not be downgraded.
async fn resolve_effective_provider(
    provider: &str,
    provider_explicit: bool,
    root: Option<&str>,
) -> Result<String> {
    if provider_explicit {
        return Ok(provider.to_string());
    }
    // "auto" is already resolved by resolve_auto_selection; if we still
    // see it here something is off — leave it alone so the normal error
    // path catches it.
    if provider == "auto" {
        return Ok(provider.to_string());
    }

    let mut on_downgrade = |from: &str, to: &str, reason: &str| {
        log::warn!("Downgrading provider: {from} → {to} ({reason})");
    };
    let (_agent, effective) = AgentFactory::create_with_fallback(
        provider,
        false,
        None,
        None,
        root.map(String::from),
        false,
        Vec::new(),
        &mut on_downgrade,
    )
    .await?;
    Ok(effective)
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
            .map(|m| format!(" with model {m}"))
            .unwrap_or_default();
        println!(
            "\x1b[32m✓\x1b[0m Auto-selected: {}{}",
            params.agent_name, model_info
        );
    }

    Ok(())
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
    max_turns: Option<u32>,
    mcp_config: Option<String>,
    env_vars: Vec<(String, String)>,
    headless: bool,
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

    if p.headless {
        agent.set_headless(true);
    }

    let output_fmt_clone = p.output_format.clone();
    agent.set_output_format(p.output_format);

    if let Some(turns) = p.max_turns {
        agent.set_max_turns(turns);
    }

    if !p.env_vars.is_empty() {
        agent.set_env_vars(p.env_vars);
    }

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
        if p.mcp_config.is_some() {
            claude_agent.set_mcp_config(p.mcp_config);
        }

        // Set up event handler for streaming output (text or stream-json modes)
        let is_stream_json = output_fmt_clone.as_deref() == Some("stream-json");
        claude_agent.set_event_handler(Box::new(move |event, verbose| {
            use crate::output::{ContentBlock, Event};
            if is_stream_json {
                // Output as unified NDJSON
                if let Ok(json) = serde_json::to_string(event) {
                    println!("{json}");
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
                                println!("{formatted}");
                            }
                        } else if let Event::AssistantMessage { content, .. } = event {
                            for block in content {
                                if let ContentBlock::Text { text } = block {
                                    print!("{text}");
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

    crate::logging::finish_spinner_quiet(&spinner);
    debug!("Agent configuration complete");

    Ok((agent, output_fmt_clone))
}

/// Context for executing an action.
struct ExecutionContext<'a> {
    provider: &'a str,
    json_mode: bool,
    json_schema: &'a Option<serde_json::Value>,
    output_fmt: Option<&'a str>,
    show_usage: bool,
    verbose: bool,
    /// True when `--exit` was set; forces the Run path to interactive
    /// regardless of `json_mode + prompt`.
    exit_active: bool,
    /// Usage-limit / auto-resume config, loaded once from `zag.toml`. Wraps
    /// the foreground auto-resume loop in the `Exec` and `Run` (JSON)
    /// branches — see `zag_orch::usage_resume::run_with_auto_resume`.
    usage_cfg: zag_agent::usage_limits::UsageLimitConfig,
    /// Root override (e.g. from `--root`) so the auto-resume loop can
    /// resolve the right state dir for `zag usage list`.
    root: Option<&'a str>,
}

/// Execute the requested action.
/// Returns `Ok(true)` if agent reported success (or no output), `Ok(false)` if agent reported an error.
async fn execute_action(
    action: Commands,
    agent: &mut (dyn crate::agent::Agent + Send + Sync),
    ctx: &ExecutionContext<'_>,
    log_writer: Option<&crate::session_log::SessionLogWriter>,
) -> Result<bool> {
    match action {
        Commands::Run {
            prompt,
            resume,
            continue_session,
            ..
        } => {
            if resume.is_some() || continue_session {
                if let Some(ref session_id) = resume {
                    info!("Resuming session {session_id}");
                } else {
                    info!("Resuming latest session");
                }

                agent
                    .run_resume(resume.as_deref(), continue_session)
                    .await?;
            } else if !ctx.exit_active && ctx.json_mode && prompt.is_some() {
                info!("Starting non-interactive session (JSON mode)");
                let wrapped = if ctx.provider != "claude" {
                    let w = prompt.as_deref().map(wrap_prompt_for_json);
                    if let Some(ref wp) = w {
                        debug!("JSON-wrapped run prompt: {wp}");
                    }
                    w
                } else {
                    debug!("Run prompt (JSON mode, Claude): {prompt:?}");
                    None
                };
                let run_prompt = wrapped
                    .as_deref()
                    .or(prompt.as_deref())
                    .unwrap_or("")
                    .to_string();
                let usage_cfg = ctx.usage_cfg.clone();
                let agent_output = zag_orch::usage_resume::run_with_auto_resume(
                    agent,
                    ctx.provider,
                    run_prompt,
                    None,
                    &usage_cfg,
                    log_writer,
                    ctx.root,
                )
                .await?;
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
        Commands::Exec { prompt, resume, .. } => {
            let run_prompt = if ctx.json_mode && ctx.provider != "claude" {
                let wrapped = wrap_prompt_for_json(&prompt);
                debug!("JSON-wrapped prompt: {wrapped}");
                wrapped
            } else {
                debug!("Exec prompt: {prompt}");
                prompt.clone()
            };

            // Load usage-limit config once so the auto-resume loop honors
            // user overrides (resume_message, fallback_secs, extra_patterns).
            let usage_cfg = ctx.usage_cfg.clone();

            info!(
                "{} (auto-resume {})",
                if resume.is_some() {
                    "Resuming session with prompt"
                } else {
                    "Starting non-interactive session"
                },
                if usage_cfg.enabled_for(ctx.provider) {
                    "enabled"
                } else {
                    "disabled"
                }
            );

            let agent_output = zag_orch::usage_resume::run_with_auto_resume(
                agent,
                ctx.provider,
                run_prompt,
                resume.clone(),
                &usage_cfg,
                log_writer,
                ctx.root,
            )
            .await?;

            let agent_success = agent_output.as_ref().map(|o| !o.is_error).unwrap_or(true);

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

            return Ok(agent_success);
        }
        _ => unreachable!(),
    }

    Ok(true)
}

/// Log configuration details at debug level.
fn log_config_details(params: &AgentActionParams) {
    if let Some(ref m) = params.model {
        debug!("Model specified: {m}");
    }
    if let Some(ref r) = params.root {
        debug!("Root directory: {r}");
    }
    if params.auto_approve {
        debug!("Auto-approve enabled");
    }
    if let Some(ref sp) = params.system_prompt {
        debug!("System prompt: {sp}");
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
        Commands::Usage { .. } => "usage",
        Commands::Capability { .. } => "capability",
        Commands::Discover { .. } => "discover",
        Commands::Listen { .. } => "listen",
        Commands::Man { .. } => "man",
        Commands::Skills { .. } => "skills",
        Commands::Mcp { .. } => "mcp",
        Commands::Ps { .. } => "ps",
        Commands::Search { .. } => "search",
        Commands::Input { .. } => "input",
        Commands::Broadcast { .. } => "broadcast",
        Commands::Whoami { .. } => "whoami",
        Commands::Env { .. } => "env",
        Commands::Collect { .. } => "collect",
        Commands::Status { .. } => "status",
        Commands::Wait { .. } => "wait",
        Commands::Spawn { .. } => "spawn",
        Commands::Pipe { .. } => "pipe",
        Commands::Events { .. } => "events",
        Commands::Cancel { .. } => "cancel",
        Commands::Summary { .. } => "summary",
        Commands::Watch { .. } => "watch",
        Commands::Subscribe { .. } => "subscribe",
        Commands::Log { .. } => "log",
        Commands::Output { .. } => "output",
        Commands::Retry { .. } => "retry",
        Commands::Gc { .. } => "gc",
        Commands::Serve { .. } => "serve",
        Commands::Connect { .. } => "connect",
        Commands::Disconnect => "disconnect",
        Commands::Relay { .. } => "relay",
        Commands::User { .. } => "user",
        Commands::Plan { .. } => "plan",
    }
}

fn action_prompt(action: &Commands) -> Option<&str> {
    match action {
        Commands::Run { prompt, .. } => prompt.as_deref(),
        Commands::Exec { prompt, .. } => Some(prompt.as_str()),
        _ => None,
    }
}

fn should_enable_live_session_logs(action: &Commands, json_mode: bool, exit_active: bool) -> bool {
    matches!(action, Commands::Run { .. }) && (exit_active || !json_mode)
}

pub(crate) async fn run_agent_action(mut params: AgentActionParams) -> Result<()> {
    resolve_auto_selection(&mut params).await?;

    // If the user did not pin a provider with `-p`, and we're not resuming
    // an existing session, walk the fallback tier list and pick the first
    // provider that actually works. This downgrades past missing binaries
    // and startup probe failures, logging each downgrade so the user can
    // see what happened.
    if !is_resume_run(&params.action) {
        let effective = resolve_effective_provider(
            &params.provider,
            params.provider_explicit,
            params.root.as_deref(),
        )
        .await?;
        if effective != params.provider {
            params.provider = effective;
            params.agent_name = crate::capitalize(&params.provider);
        }
    }

    log_config_details(&params);

    let session_metadata = std::mem::take(&mut params.session_metadata);
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
        session,
        max_turns,
        mcp_config,
        timeout,
        exit_on_failure,
        context_session,
        plan_path,
        env_vars,
        files,
        session_metadata: _,
        exit_hint,
        headless,
    } = params;

    // `--exit` is only meaningful in interactive run mode — it tells the
    // agent to terminate by calling `zag ps kill self <result>`. `exec`
    // already produces a structured output natively, so combining the
    // two would be ambiguous.
    if exit_hint.is_some() && matches!(action, Commands::Exec { .. }) {
        bail!(
            "--exit is only valid with `run` (interactive) mode. Use \
             `zag -p <provider> run --exit '<hint>' \"<prompt>\"` instead of `exec`."
        );
    }

    // `--headless` hides the provider's TUI by attaching it to a private
    // PTY. The hidden TUI cannot answer permission prompts and produces no
    // visible output, so it only makes sense with `-a` (auto-approve) and
    // `--exit` (so the run has a well-defined termination + result signal
    // via `zag ps kill self`). It also only applies to the interactive
    // `run` path — `exec` already runs non-interactively.
    if headless {
        if !auto_approve {
            bail!(
                "--headless requires -a (auto-approve): the hidden TUI cannot answer \
                 permission prompts. Re-run with `-a` or drop `--headless`."
            );
        }
        if exit_hint.is_none() {
            bail!(
                "--headless requires --exit: a hidden run needs an explicit termination \
                 and result signal (the agent should call `zag ps kill self <result>`)."
            );
        }
        if matches!(action, Commands::Exec { .. }) {
            bail!(
                "--headless only applies to `run` (interactive) mode; `exec` already runs \
                 non-interactively. Drop `--headless` or switch to `run`."
            );
        }
    }

    // Apply config fallbacks for max_turns and system_prompt
    let config = Config::load(root.as_deref()).unwrap_or_default();
    let max_turns = max_turns.or(config.max_turns());
    let system_prompt = system_prompt.or_else(|| config.system_prompt().map(String::from));

    let is_exec = matches!(action, Commands::Exec { .. });
    let show_wrapper = !quiet && (!is_exec || verbose);

    let mut system_prompt =
        augment_system_prompt_for_json(system_prompt, json_mode, &provider, &json_schema);

    // When `--exit` is set, inject the kill-self instructions into the
    // system prompt rather than the user prompt — this keeps the user's
    // visible prompt clean in the TUI / log while the agent still gets the
    // termination protocol via the system channel.
    if let Some(ref hint) = exit_hint {
        let suffix =
            zag_agent::exit_mode::build_exit_suffix(hint.as_str(), json_mode, json_schema.as_ref());
        system_prompt = Some(match system_prompt {
            Some(existing) if !existing.is_empty() => format!("{existing}\n\n{suffix}"),
            _ => suffix,
        });
    }

    if let Err(e) = skills::setup_skills(&provider, &mut system_prompt) {
        log::warn!("Failed to set up skills: {e}");
    }

    if let Err(e) = mcp::setup_mcp(&provider, root.as_deref()) {
        log::warn!("Failed to set up MCP servers: {e}");
    }

    if let Some(ref sp) = system_prompt {
        debug!("Effective system prompt: {sp}");
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
        match &mut action {
            Commands::Run {
                resume,
                continue_session,
                ..
            }
            | Commands::Exec {
                resume,
                continue_session,
                ..
            } => {
                *resume = Some(native_id);
                *continue_session = false;
            }
            _ => {}
        }
    }

    let is_resume = is_resume_run(&action);
    let plain = setup_plain_session(
        is_new_interactive_run(&action, json_mode, exit_hint.is_some()),
        &root,
        &session,
    );
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
        is_resume,
        &root,
        show_wrapper,
        wrapper_session_id.clone(),
    )?;
    let sb = setup_sandbox(&sandbox_flag, is_resume, &root, wrapper_session_id.clone())?;

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
        debug!("Output format: {o}");
    }
    if let Some(ref i) = input_format {
        debug!("Input format: {i}");
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
            max_turns,
            mcp_config,
            env_vars,
            headless,
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
            println!("\x1b[32m✓\x1b[0m Sandbox configured: {name}");
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
            println!("\x1b[32m✓\x1b[0m Sandbox configured: {name}");
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
        &session_metadata,
    );

    // Register the process entry, build the `ZAG_*` env vars, and wire the
    // on_spawn pid retarget — all via the shared `process_registration`
    // helper. The helper is the single source of truth for this sequence;
    // library callers (e.g. `zig run` interactive steps via
    // `AgentBuilder::register_process`) get the same wiring.
    //
    // Previously this block called `unsafe std::env::set_var` to leak
    // `ZAG_PROCESS_ID` etc. into zag's own process env so the spawned agent
    // would inherit them. The helper now passes the env vars directly to
    // the agent subprocess via `Agent::set_env_vars`, which the providers
    // forward to their `Command::env(...)` — so zag's own env stays clean
    // and there's no `unsafe` block.
    //
    // Pid retargeting: the entry is registered with `pid = zag's own pid`
    // up-front so `zag ps show` works while the agent is still booting;
    // once the agent subprocess spawns, the on_spawn hook flips the entry
    // to point at the agent child. That way `zag ps kill self` SIGTERMs
    // the agent (which dies cleanly so the orchestrator can move on),
    // not the parent zag wrapper.
    let proc_session_id = wt
        .session_id
        .clone()
        .or_else(|| sb.session_id.clone())
        .or_else(|| plain.session_id.clone());
    let proc_prompt = action_prompt(&action).map(|p| p.chars().take(100).collect::<String>());
    let proc_cmd = command_name(&action).to_string();

    let registration = zag_agent::process_registration::register(
        zag_agent::process_registration::RegisterOptions {
            provider: &provider,
            model: &persisted_model,
            command: &proc_cmd,
            prompt_preview: proc_prompt.as_deref(),
            session_id: proc_session_id.as_deref(),
            session_name: session_metadata.name.as_deref(),
            root: root.as_deref(),
        },
    );

    agent.set_env_vars(registration.env_vars().to_vec());
    agent.set_on_spawn_hook(registration.on_spawn_hook());

    // Write lifecycle started marker and prune old markers
    let lifecycle_session_id = wt
        .session_id
        .as_deref()
        .or(sb.session_id.as_deref())
        .or(plain.session_id.as_deref())
        .unwrap_or(&log_session_id)
        .to_string();
    zag_orch::lifecycle::write_started_marker(&lifecycle_session_id);
    zag_orch::lifecycle::prune_old_markers();

    // Echo session ID for `agent listen` usage
    if show_wrapper {
        let display_session_id = wt
            .session_id
            .as_deref()
            .or(sb.session_id.as_deref())
            .or(plain.session_id.as_deref())
            .unwrap_or(&log_session_id);
        println!("\x1b[33m>\x1b[0m Session: {display_session_id}");
        println!("\x1b[33m>\x1b[0m Listen:  zag listen {display_session_id}");
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
        should_enable_live_session_logs(&action, json_mode, exit_hint.is_some()),
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

    // Resolve --context: prepend another session's result to the prompt
    if let Some(ref ctx_session_id) = context_session {
        let context_text =
            zag_orch::collect::extract_last_assistant_message(ctx_session_id, root.as_deref());
        if let Some(context_text) = context_text {
            let prefix = format!(
                "Context from previous session ({ctx_session_id}):\n\n{context_text}\n\n---\n\n"
            );
            match &mut action {
                Commands::Exec { prompt, .. } => {
                    *prompt = format!("{prefix}{prompt}");
                }
                Commands::Run {
                    prompt: Some(p), ..
                } => {
                    *p = format!("{prefix}{p}");
                }
                _ => {}
            }
        }
    }

    // Resolve --plan: prepend a plan file's content to the prompt
    if let Some(ref plan_file) = plan_path {
        let plan_content = std::fs::read_to_string(plan_file)
            .with_context(|| format!("Failed to read plan file: {plan_file}"))?;
        let prefix =
            format!("Implementation plan:\n\n{plan_content}\n\n---\n\nFollow the plan above.\n\n");
        match &mut action {
            Commands::Exec { prompt, .. } => {
                *prompt = format!("{prefix}{prompt}");
            }
            Commands::Run {
                prompt: Some(p), ..
            } => {
                *p = format!("{prefix}{p}");
            }
            Commands::Run { prompt, .. } => {
                *prompt = Some(prefix);
            }
            _ => {}
        }
    }

    // Resolve --file: prepend file attachments to the prompt
    if !files.is_empty() {
        let attachments = files
            .iter()
            .map(|f| zag_agent::attachment::Attachment::from_path(std::path::Path::new(f)))
            .collect::<Result<Vec<_>>>()?;
        let prefix = zag_agent::attachment::format_attachments_prefix(&attachments);
        match &mut action {
            Commands::Exec { prompt, .. } => {
                *prompt = format!("{prefix}{prompt}");
            }
            Commands::Run {
                prompt: Some(p), ..
            } => {
                *p = format!("{prefix}{p}");
            }
            Commands::Run { prompt, .. } => {
                *prompt = Some(prefix);
            }
            _ => {}
        }
    }

    // Persist the --exit constraints into the session store so that
    // `zag ps kill self <result>` can validate them at termination. The
    // exit-mode instructions themselves were injected into the system
    // prompt earlier (before agent creation) so the user's typed prompt
    // stays clean in the TUI / session log.
    if let Some(ref hint) = exit_hint {
        let sid_for_exit = wt
            .session_id
            .as_deref()
            .or(sb.session_id.as_deref())
            .or(plain.session_id.as_deref());
        if let Some(sid) = sid_for_exit {
            let mut store =
                zag_agent::session::SessionStore::load(root.as_deref()).unwrap_or_default();
            if let Some(entry) = store.sessions.iter_mut().find(|e| e.session_id == sid) {
                entry.exit = Some(zag_agent::exit_mode::ExitConstraints {
                    hint: Some(hint.clone()),
                    json_mode,
                    schema: json_schema.clone(),
                });
                if let Err(e) = store.save(root.as_deref()) {
                    log::warn!("Failed to persist --exit session metadata: {e}");
                }
            }
        }
    }

    let is_worktree_session = wt.is_worktree_session;
    let is_interactive_worktree = wt.is_worktree_session && matches!(action, Commands::Run { .. });
    let is_interactive_sandbox = sb.is_sandbox_session && matches!(action, Commands::Run { .. });
    let is_interactive_run = matches!(action, Commands::Run { .. });

    let usage_cfg = Config::load(root.as_deref())
        .map(|c| c.usage_limits)
        .unwrap_or_default();
    let exec_ctx = ExecutionContext {
        provider: &provider,
        json_mode,
        json_schema: &json_schema,
        output_fmt: output_fmt_clone.as_deref(),
        show_usage,
        verbose,
        exit_active: exit_hint.is_some(),
        usage_cfg,
        root: root.as_deref(),
    };
    let action_future = execute_action(
        action,
        &mut *agent,
        &exec_ctx,
        Some(log_coordinator.writer()),
    );
    let action_result = if let Some(ref timeout_str) = timeout {
        let duration = zag_orch::duration::parse_duration(timeout_str)?;
        match tokio::time::timeout(duration, action_future).await {
            Ok(r) => r,
            Err(_) => Err(anyhow::anyhow!("Agent timed out after {timeout_str}")),
        }
    } else {
        action_future.await
    };

    // Always run agent cleanup regardless of action result to prevent resource leaks.
    debug!("Cleaning up agent resources");
    if let Err(cleanup_err) = agent.cleanup().await {
        log::warn!("Agent cleanup failed: {cleanup_err}");
    }

    // When `--exit` is active, the agent's intended termination path is
    // `zag ps kill self <result>`, which SIGTERMs the agent process —
    // making the provider exit with 143. That's not a crash, so look for
    // a `SessionResult` event in the log: if one is present, the kill was
    // the agent's own clean exit and we should treat it as success.
    let exit_mode_result: Option<String> = if action_result.is_err() && exit_hint.is_some() {
        let sid = wt
            .session_id
            .as_deref()
            .or(sb.session_id.as_deref())
            .or(plain.session_id.as_deref());
        sid.and_then(|id| zag_orch::collect::extract_session_result(id, root.as_deref()))
    } else {
        None
    };

    let agent_success = match (&action_result, &exit_mode_result) {
        (Err(_), Some(_)) => true,
        (Err(err), None) => {
            // Extract structured error info if available
            let process_err = err.downcast_ref::<zag_agent::process::ProcessError>();
            let exit_code = process_err.and_then(|pe| pe.exit_code).unwrap_or(1);

            registration.update_status("killed", Some(exit_code));
            // Use ok() to avoid masking the original error if log finishing also fails
            if let Err(log_err) = log_coordinator.finish(false, Some(err.to_string())).await {
                log::warn!("Failed to finish session log: {log_err}");
            }
            zag_orch::lifecycle::write_ended_marker(&lifecycle_session_id, false, Some(exit_code));

            // Show the error to the user on stderr
            eprintln!("\x1b[31merror\x1b[0m: {err}");
            eprintln!("\x1b[2mRun with --debug for full details\x1b[0m");

            // Exit with structured code: 2 for provider process crash
            if process_err.is_some() {
                std::process::exit(EXIT_PROVIDER_ERROR);
            }
            return Err(anyhow::anyhow!(err.to_string()));
        }
        (Ok(success), _) => *success,
    };

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
    let final_exit_code = if agent_success { 0 } else { EXIT_AGENT_FAILURE };
    log_coordinator.finish(agent_success, None).await?;
    zag_orch::lifecycle::write_ended_marker(
        &lifecycle_session_id,
        agent_success,
        Some(final_exit_code),
    );

    registration.update_status("exited", Some(final_exit_code));

    if !agent_success {
        eprintln!("\x1b[31merror\x1b[0m: agent exited with failure");
        eprintln!("\x1b[2mRun with --debug for full details\x1b[0m");
    }

    if show_wrapper {
        println!("\x1b[32m✓\x1b[0m Session terminated");
    }

    // Surface the captured `--exit` result. When the wrapper UI is on,
    // print it under a labelled header so it's visually distinct from
    // the rest of the session output. When quiet, print just the raw
    // result so the output is pipe-friendly.
    if let Some(ref result) = exit_mode_result {
        if show_wrapper {
            println!("\x1b[36m> Result:\x1b[0m {result}");
        } else {
            println!("{result}");
        }
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
                        log::warn!("Failed to remove worktree: {e}");
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

    // Exit with code 1 if agent reported failure and --exit-on-failure is set
    if exit_on_failure && !agent_success {
        std::process::exit(EXIT_AGENT_FAILURE);
    }

    Ok(())
}
