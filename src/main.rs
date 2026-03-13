mod agent;
mod auto_selector;
mod claude;
mod codex;
mod config;
mod copilot;
mod factory;
mod gemini;
mod json_validation;
mod logging;
mod output;
mod process;
mod session;
mod worktree;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use config::Config;
use factory::AgentFactory;
use log::{debug, info};

#[derive(Parser)]
#[command(name = "agent")]
#[command(about = "A wrapper for different AI agents")]
struct Cli {
    /// Enable debug logging
    #[arg(short, long, global = true)]
    debug: bool,

    /// Quiet mode - disable all logging except agent output
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Verbose mode - show detailed formatted output with icons and status messages
    #[arg(short = 'v', long, global = true)]
    verbose: bool,

    /// Show token usage statistics (only applies to JSON output mode)
    #[arg(long, global = true)]
    show_usage: bool,

    /// Provider to use (claude, codex, gemini, copilot, auto)
    #[arg(short = 'p', long, global = true)]
    provider: Option<String>,

    /// System prompt to configure agent behavior
    #[arg(short, long, global = true)]
    system_prompt: Option<String>,

    /// Model to use (agent-specific, size alias: small/medium/large, or auto)
    #[arg(short, long, global = true)]
    model: Option<String>,

    /// Root directory to run the agent in
    #[arg(short, long, global = true)]
    root: Option<String>,

    /// Auto-approve all actions (skip permission prompts)
    #[arg(short = 'a', long, global = true)]
    auto_approve: bool,

    /// Additional directories to include
    #[arg(long = "add-dir", global = true)]
    add_dirs: Vec<String>,

    /// Create a git worktree for this session (optionally specify a name)
    #[arg(short = 'w', long, global = true)]
    worktree: Option<Option<String>>,

    /// Request JSON output from the agent
    #[arg(long, global = true)]
    json: bool,

    /// JSON schema for structured output (file path or inline JSON string)
    #[arg(long, global = true, value_name = "SCHEMA")]
    json_schema: Option<String>,

    /// Stream JSON events (NDJSON format) — sets output format to stream-json
    #[arg(long, global = true)]
    json_stream: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start an interactive session
    Run {
        /// Initial prompt for the session
        prompt: Option<String>,
    },
    /// Run non-interactively (print output and exit)
    Exec {
        /// The prompt to send to the agent
        prompt: String,

        /// Output format (text, json, json-pretty, stream-json, native-json)
        #[arg(short = 'o', long)]
        output: Option<String>,

        /// Input format (text, stream-json) - Claude only
        #[arg(short = 'i', long)]
        input_format: Option<String>,
    },
    /// Resume a previous session
    Resume {
        /// Session ID to resume
        session_id: Option<String>,

        /// Resume the most recent session
        #[arg(long)]
        last: bool,
    },
    /// Review code changes (uses Codex under the hood)
    Review {
        /// Review staged/unstaged/untracked changes
        #[arg(long)]
        uncommitted: bool,

        /// Review against a base branch
        #[arg(long)]
        base: Option<String>,

        /// Review changes from a specific commit
        #[arg(long)]
        commit: Option<String>,

        /// Optional title for the review summary
        #[arg(long)]
        title: Option<String>,
    },
    /// View or set configuration values
    Config {
        /// Config key and value (e.g., "provider claude" or "provider=claude")
        args: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // In exec mode without --verbose, suppress info-level logging (treat as quiet for the logger)
    let is_exec = matches!(cli.command, Commands::Exec { .. });
    let effective_quiet = cli.quiet || (is_exec && !cli.verbose);

    // Initialize logging
    logging::init(cli.debug, effective_quiet);
    debug!("Debug logging enabled");

    let show_usage = cli.show_usage;
    let quiet = cli.quiet;
    let verbose = cli.verbose;

    // --json-schema implies --json
    let json_mode = cli.json || cli.json_schema.is_some();
    let json_schema = cli.json_schema;
    let json_stream = cli.json_stream;

    // Validate --json-stream is mutually exclusive with --json/--json-schema
    if json_stream && json_mode {
        bail!("--json-stream cannot be combined with --json or --json-schema");
    }

    // Validate --json-stream usage (same restrictions as --json)
    if json_stream {
        match &cli.command {
            Commands::Resume { .. } => bail!("--json-stream cannot be used with resume"),
            Commands::Review { .. } => bail!("--json-stream cannot be used with review"),
            Commands::Config { .. } => bail!("--json-stream cannot be used with config"),
            Commands::Run { prompt } if prompt.is_none() => {
                bail!("--json-stream requires a prompt (use exec or run with a prompt)")
            }
            _ => {}
        }
    }

    // Validate --json/--json-schema usage
    if json_mode {
        match &cli.command {
            Commands::Resume { .. } => bail!("--json/--json-schema cannot be used with resume"),
            Commands::Review { .. } => bail!("--json/--json-schema cannot be used with review"),
            Commands::Config { .. } => bail!("--json/--json-schema cannot be used with config"),
            Commands::Run { prompt } if prompt.is_none() => {
                bail!("--json/--json-schema requires a prompt (use exec or run with a prompt)")
            }
            _ => {}
        }

        // Validate schema is valid JSON if provided
        if let Some(ref schema_str) = json_schema {
            // Try to load as file first, then as inline JSON
            let schema_json = if std::path::Path::new(schema_str).exists() {
                let content = std::fs::read_to_string(schema_str).map_err(|e| {
                    anyhow::anyhow!("Failed to read JSON schema file '{}': {}", schema_str, e)
                })?;
                serde_json::from_str::<serde_json::Value>(&content).map_err(|e| {
                    anyhow::anyhow!("Invalid JSON in schema file '{}': {}", schema_str, e)
                })?
            } else {
                serde_json::from_str::<serde_json::Value>(schema_str)
                    .map_err(|e| anyhow::anyhow!("Invalid JSON schema: {}", e))?
            };
            // Validate it's a valid JSON schema by checking it has a type field
            debug!(
                "JSON schema loaded: {} bytes",
                serde_json::to_string(&schema_json)
                    .unwrap_or_default()
                    .len()
            );
        }
    }

    // Validate --worktree usage (ignored with resume — worktree comes from session mapping)
    if cli.worktree.is_some() {
        match &cli.command {
            Commands::Review { .. } => bail!("--worktree cannot be used with review"),
            Commands::Config { .. } => bail!("--worktree cannot be used with config"),
            _ => {}
        }
    }

    // Validate auto provider/model usage
    let is_auto_provider = cli.provider.as_deref() == Some("auto");
    let is_auto_model = cli.model.as_deref() == Some("auto");
    if is_auto_provider || is_auto_model {
        match &cli.command {
            Commands::Resume { .. } => bail!("auto cannot be used with resume"),
            Commands::Review { .. } => bail!("auto cannot be used with review"),
            Commands::Config { .. } => bail!("auto cannot be used with config"),
            _ => {}
        }
    }

    match cli.command {
        Commands::Config { args } => {
            run_config(args, cli.root.as_deref())?;
        }
        Commands::Review {
            uncommitted,
            base,
            commit,
            title,
        } => {
            run_review(ReviewParams {
                uncommitted,
                base,
                commit,
                title,
                system_prompt: cli.system_prompt,
                model: cli.model,
                root: cli.root,
                auto_approve: cli.auto_approve,
                add_dirs: cli.add_dirs,
                quiet,
            })
            .await?;
        }
        action => {
            let provider = resolve_provider(cli.provider.as_deref(), cli.root.as_deref())?;
            let display_name = capitalize(&provider);
            run_agent_action(AgentActionParams {
                agent_name: display_name,
                provider,
                action,
                system_prompt: cli.system_prompt,
                model: cli.model,
                root: cli.root,
                auto_approve: cli.auto_approve,
                add_dirs: cli.add_dirs,
                show_usage,
                quiet,
                verbose,
                worktree: cli.worktree,
                json_mode,
                json_schema,
                json_stream,
            })
            .await?;
        }
    }

    Ok(())
}

/// Resolve the provider name from CLI flag, config, or default.
fn resolve_provider(flag: Option<&str>, root: Option<&str>) -> Result<String> {
    if let Some(p) = flag {
        let p = p.to_lowercase();
        if !Config::VALID_PROVIDERS.contains(&p.as_str()) {
            bail!(
                "Invalid provider '{}'. Available: {}",
                p,
                Config::VALID_PROVIDERS.join(", ")
            );
        }
        return Ok(p);
    }

    let config = Config::load(root).unwrap_or_default();
    if let Some(p) = config.provider() {
        return Ok(p.to_string());
    }

    Ok("claude".to_string())
}

/// Capitalize the first letter of a string.
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Handle `agent config` subcommand.
fn run_config(args: Vec<String>, root: Option<&str>) -> Result<()> {
    if args.is_empty() {
        // Print full config file contents
        let path = Config::config_path(root);
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            print!("{}", content);
        } else {
            println!("No config file found at {}", path.display());
            println!("Run any agent command to create a default config.");
        }
        return Ok(());
    }

    // Parse key=value or key value
    let (key, value) = if args.len() == 1 {
        // Single arg — check for key=value
        if let Some((k, v)) = args[0].split_once('=') {
            (k.to_string(), v.to_string())
        } else {
            bail!(
                "Missing value. Usage: agent config {}=<value> or agent config {} <value>",
                args[0],
                args[0]
            );
        }
    } else {
        // Two args: key value
        (args[0].clone(), args[1].clone())
    };

    let mut config = Config::load(root).unwrap_or_default();
    config.set_value(&key, &value)?;
    config.save(root)?;
    println!("{} = {}", key, value);
    Ok(())
}

struct AgentActionParams {
    agent_name: String,
    provider: String,
    action: Commands,
    system_prompt: Option<String>,
    model: Option<String>,
    root: Option<String>,
    auto_approve: bool,
    add_dirs: Vec<String>,
    show_usage: bool,
    quiet: bool,
    verbose: bool,
    worktree: Option<Option<String>>,
    json_mode: bool,
    json_schema: Option<String>,
    json_stream: bool,
}

async fn run_agent_action(mut params: AgentActionParams) -> Result<()> {
    // Handle auto provider/model selection before anything else
    let is_auto_provider = params.provider == "auto";
    let is_auto_model = params.model.as_deref() == Some("auto");

    if is_auto_provider || is_auto_model {
        // Extract the prompt from the action for auto-selection
        let task_prompt = match &params.action {
            Commands::Run { prompt } => prompt.as_deref(),
            Commands::Exec { prompt, .. } => Some(prompt.as_str()),
            _ => None,
        };

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
            // Provider changed, clear model so the new provider's default is used
            params.model = None;
        }

        params.agent_name = capitalize(&params.provider);

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
    }

    let AgentActionParams {
        agent_name,
        provider,
        action,
        system_prompt,
        model,
        root,
        auto_approve,
        add_dirs,
        show_usage,
        quiet,
        verbose,
        worktree: worktree_flag,
        json_mode,
        json_schema,
        json_stream,
    } = params;
    let is_exec = matches!(action, Commands::Exec { .. });
    let show_wrapper = !quiet && (!is_exec || verbose);
    // Log configuration details
    if let Some(ref m) = model {
        debug!("Model specified: {}", m);
    }
    if let Some(ref r) = root {
        debug!("Root directory: {}", r);
    }
    if auto_approve {
        debug!("Auto-approve enabled");
    }
    if let Some(ref sp) = system_prompt {
        debug!("System prompt: {}", sp);
    }
    if !add_dirs.is_empty() {
        debug!("Additional directories: {:?}", add_dirs);
    }
    if worktree_flag.is_some() {
        debug!("Worktree mode enabled");
    }
    if json_mode {
        debug!("JSON output mode enabled");
    }

    // Load and resolve JSON schema if provided
    let resolved_schema: Option<serde_json::Value> = if let Some(ref schema_str) = json_schema {
        let json = if std::path::Path::new(schema_str).exists() {
            let content = std::fs::read_to_string(schema_str)?;
            serde_json::from_str(&content)?
        } else {
            serde_json::from_str(schema_str)?
        };
        Some(json)
    } else {
        None
    };

    // Augment system prompt for JSON mode (non-Claude agents need this;
    // Claude gets --json-schema natively but also benefits from prompt guidance)
    let system_prompt = if json_mode && provider != "claude" {
        let mut prompt = system_prompt.unwrap_or_default();
        if let Some(ref schema) = resolved_schema {
            let schema_str = serde_json::to_string_pretty(schema).unwrap_or_default();
            prompt.push_str(&format!(
                "\n\nYou MUST respond with valid JSON only. No markdown fences, no explanations. \
                 Your response must conform to this JSON schema:\n{}",
                schema_str
            ));
        } else {
            prompt.push_str(
                "\n\nYou MUST respond with valid JSON only. No markdown fences, no explanations.",
            );
        }
        Some(prompt)
    } else {
        system_prompt
    };

    // Generate session ID and worktree name for worktree sessions
    let is_worktree_session = worktree_flag.is_some() && !matches!(action, Commands::Resume { .. });
    let session_id = if is_worktree_session {
        Some(uuid::Uuid::new_v4().to_string())
    } else {
        None
    };
    let worktree_name = if is_worktree_session {
        let name = worktree_flag
            .as_ref()
            .unwrap()
            .as_deref()
            .map(String::from)
            .unwrap_or_else(worktree::generate_name);
        Some(name)
    } else {
        None
    };

    // Handle worktree creation for non-Claude providers
    let effective_root = if is_worktree_session {
        if provider != "claude" {
            let repo_root = worktree::git_repo_root(root.as_deref())?;
            let name = worktree_name.as_deref().unwrap();
            let wt_path = worktree::create_worktree(&repo_root, name)?;
            if show_wrapper {
                println!("\x1b[32m✓\x1b[0m Worktree created at {}", wt_path.display());
            }
            Some(wt_path.to_string_lossy().to_string())
        } else {
            root.clone()
        }
    } else {
        root.clone()
    };

    // Compute worktree path for session mapping
    let worktree_path: Option<String> = if is_worktree_session {
        if provider == "claude" {
            // Claude creates worktrees at <repo-root>/.claude/worktrees/<name>
            let repo_root = worktree::git_repo_root(root.as_deref())?;
            let name = worktree_name.as_deref().unwrap();
            Some(
                repo_root
                    .join(".claude")
                    .join("worktrees")
                    .join(name)
                    .to_string_lossy()
                    .to_string(),
            )
        } else {
            // Non-Claude worktrees are at effective_root (already created above)
            effective_root.clone()
        }
    } else {
        None
    };

    // Extract output/input format from exec action
    let (output_format, input_format) = match &action {
        Commands::Exec {
            output,
            input_format,
            ..
        } => (output.clone(), input_format.clone()),
        _ => (None, None),
    };

    if let Some(ref o) = output_format {
        debug!("Output format: {}", o);
    }
    if let Some(ref i) = input_format {
        debug!("Input format: {}", i);
    }

    // Create agent with spinner (skip in exec mode unless verbose)
    let spinner = if show_wrapper {
        logging::spinner(format!("Initializing {} agent", agent_name))
    } else {
        let pb = indicatif::ProgressBar::new_spinner();
        pb.set_draw_target(indicatif::ProgressDrawTarget::hidden());
        pb
    };
    let mut agent = AgentFactory::create(
        &provider,
        system_prompt,
        model,
        effective_root,
        auto_approve,
        add_dirs,
    )?;

    // Set output format if specified
    let output_fmt_clone = output_format.clone();
    agent.set_output_format(output_format);

    // Set verbose and input format for Claude
    if provider == "claude"
        && let Some(claude_agent) = agent.as_any_mut().downcast_mut::<crate::claude::Claude>()
    {
        claude_agent.set_verbose(verbose);
        if let Some(input_fmt) = input_format {
            claude_agent.set_input_format(Some(input_fmt));
        }
    }

    // Set worktree passthrough and session ID for Claude (it handles worktrees natively)
    if is_worktree_session
        && provider == "claude"
        && let Some(claude_agent) = agent.as_any_mut().downcast_mut::<crate::claude::Claude>()
    {
        claude_agent.set_worktree(worktree_name.clone());
        if let Some(ref sid) = session_id {
            claude_agent.set_session_id(sid.clone());
        }
    }

    // Set JSON schema on Claude agent (native --json-schema support)
    if json_mode
        && provider == "claude"
        && let Some(claude_agent) = agent.as_any_mut().downcast_mut::<crate::claude::Claude>()
        && let Some(ref schema) = resolved_schema
    {
        let schema_str = serde_json::to_string(schema).unwrap_or_default();
        claude_agent.set_json_schema(Some(schema_str));
    }

    // Force output capture when JSON mode is active so we get AgentOutput back for validation
    let user_output_format = output_fmt_clone.clone();
    if json_mode && user_output_format.is_none() {
        agent.set_output_format(Some("json".to_string()));
        // Non-Claude agents need capture_output explicitly set (Claude handles it via output_format)
        if provider != "claude" {
            agent.set_capture_output(true);
        }
    }

    // --json-stream: set output format to stream-json (unless user already specified -o)
    if json_stream && user_output_format.is_none() {
        agent.set_output_format(Some("stream-json".to_string()));
    }

    logging::finish_spinner_quiet(&spinner);
    debug!("Agent configuration complete");

    // Get the actual model being used (after resolution)
    let model_name = agent.get_model();
    let auto_approve_suffix = if auto_approve { " (auto approve)" } else { "" };

    if show_wrapper {
        println!(
            "\x1b[32m✓\x1b[0m {} initialized with model {}{}",
            agent_name, model_name, auto_approve_suffix
        );
    }

    // Save session-worktree mapping before execution (so it survives Ctrl+C)
    if let (Some(sid), Some(wt_path), Some(wt_name)) = (&session_id, &worktree_path, &worktree_name)
    {
        let mut store = session::SessionStore::load(root.as_deref()).unwrap_or_default();
        store.add(session::SessionEntry {
            session_id: sid.clone(),
            provider: provider.clone(),
            worktree_path: wt_path.clone(),
            worktree_name: wt_name.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
        });
        if let Err(e) = store.save(root.as_deref()) {
            log::warn!("Failed to save session mapping: {}", e);
        }
        debug!("Saved session mapping: {} -> {}", sid, wt_path);
    }

    // Track whether this was an interactive worktree session (for cleanup prompt)
    let is_interactive_worktree = is_worktree_session && matches!(action, Commands::Run { .. });

    // Track resume worktree path for cleanup prompt after resume
    let mut resume_worktree_info: Option<(String, String)> = None; // (session_id, worktree_path)

    match action {
        Commands::Run { prompt } => {
            if json_mode && prompt.is_some() {
                // JSON mode with prompt — run non-interactively for output capture
                info!("Starting non-interactive session (JSON mode)");
                let agent_output = agent.run(prompt.as_deref()).await?;
                handle_json_output(agent_output, &*agent, &resolved_schema, show_usage, verbose)
                    .await?;
            } else {
                info!("Starting interactive session");
                agent.run_interactive(prompt.as_deref()).await?;
            }
        }
        Commands::Exec { prompt, .. } => {
            info!("Starting non-interactive session");
            let agent_output = agent.run(Some(&prompt)).await?;

            if json_mode {
                // JSON validation and retry loop
                handle_json_output(agent_output, &*agent, &resolved_schema, show_usage, verbose)
                    .await?;
            } else if let Some(agent_out) = agent_output {
                match output_fmt_clone.as_deref() {
                    Some("json") => {
                        let json = serde_json::to_string(&agent_out)?;
                        println!("{}", json);
                    }
                    Some("json-pretty") => {
                        let json = serde_json::to_string_pretty(&agent_out)?;
                        println!("{}", json);
                    }
                    Some("stream-json") => {
                        for event in &agent_out.events {
                            let json = serde_json::to_string(&event)?;
                            println!("{}", json);
                        }
                    }
                    _ => {
                        process_agent_output(&agent_out, show_usage, verbose)?;
                    }
                }
            }
        }
        Commands::Resume {
            session_id: resume_id,
            last,
        } => {
            // Look up worktree from session mapping
            if let Some(ref sid) = resume_id {
                let store = session::SessionStore::load(root.as_deref()).unwrap_or_default();
                if let Some(entry) = store.find_by_session_id(sid) {
                    let wt_path = std::path::Path::new(&entry.worktree_path);
                    if wt_path.exists() {
                        debug!("Resuming in worktree: {}", entry.worktree_path);
                        agent.set_root(entry.worktree_path.clone());
                        resume_worktree_info = Some((sid.clone(), entry.worktree_path.clone()));
                    } else {
                        log::warn!(
                            "Worktree no longer exists at {}, resuming without it",
                            entry.worktree_path
                        );
                        // Remove stale entry
                        let mut store = store;
                        store.remove(sid);
                        let _ = store.save(root.as_deref());
                    }
                }
            }

            info!("Resuming session");
            agent.run_resume(resume_id.as_deref(), last).await?;
        }
        _ => unreachable!(),
    }

    // Cleanup
    debug!("Cleaning up agent resources");
    agent.cleanup().await?;
    info!("Session terminated");

    // Cleanup prompt for interactive worktree sessions
    let cleanup_info = if is_interactive_worktree {
        session_id
            .as_ref()
            .zip(worktree_path.as_ref())
            .map(|(sid, wtp)| (sid.clone(), wtp.clone()))
    } else {
        resume_worktree_info
    };

    if let Some((sid, wtp)) = cleanup_info {
        prompt_worktree_cleanup(&sid, &wtp, root.as_deref())?;
    }

    Ok(())
}

/// Prompt the user whether to keep or delete a worktree after an interactive session.
fn prompt_worktree_cleanup(
    session_id: &str,
    worktree_path: &str,
    root: Option<&str>,
) -> Result<()> {
    use std::io::{self, BufRead, Write};

    println!("\n\x1b[33m>\x1b[0m Worktree at {}", worktree_path);
    print!("\x1b[33m>\x1b[0m Keep workspace? [Y/n] ");
    io::stdout().flush()?;

    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    let answer = line.trim().to_lowercase();

    if answer == "n" || answer == "no" {
        let wt_path = std::path::Path::new(worktree_path);
        if wt_path.exists() {
            match worktree::remove_worktree(wt_path) {
                Ok(()) => {
                    println!("\x1b[32m✓\x1b[0m Worktree removed");
                }
                Err(e) => {
                    log::warn!("Failed to remove worktree: {}", e);
                    println!("\x1b[31m✗\x1b[0m Failed to remove worktree: {}", e);
                }
            }
        }
        // Remove session mapping
        let mut store = session::SessionStore::load(root).unwrap_or_default();
        store.remove(session_id);
        let _ = store.save(root);
    } else {
        println!(
            "\x1b[32m✓\x1b[0m Workspace kept. Resume with: agent resume {}",
            session_id
        );
    }

    Ok(())
}

struct ReviewParams {
    uncommitted: bool,
    base: Option<String>,
    commit: Option<String>,
    title: Option<String>,
    system_prompt: Option<String>,
    model: Option<String>,
    root: Option<String>,
    auto_approve: bool,
    add_dirs: Vec<String>,
    quiet: bool,
}

async fn run_review(params: ReviewParams) -> Result<()> {
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

    debug!("Starting code review via Codex");

    let spinner = logging::spinner("Initializing Codex for review".to_string());
    let mut agent =
        AgentFactory::create("codex", system_prompt, model, root, auto_approve, add_dirs)?;
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

    codex
        .review(
            uncommitted,
            base.as_deref(),
            commit.as_deref(),
            title.as_deref(),
        )
        .await?;

    Ok(())
}

const MAX_JSON_RETRIES: usize = 3;

/// Handle JSON output mode: validate agent output and retry via session resume if invalid.
async fn handle_json_output(
    agent_output: Option<crate::output::AgentOutput>,
    agent: &(dyn crate::agent::Agent + Sync),
    schema: &Option<serde_json::Value>,
    _show_usage: bool,
    _verbose: bool,
) -> Result<()> {
    let Some(agent_out) = agent_output else {
        bail!("Agent produced no output for JSON validation");
    };

    let result_text = agent_out
        .final_result()
        .ok_or_else(|| anyhow::anyhow!("Agent output has no result text for JSON validation"))?
        .to_string();

    let session_id = if !agent_out.session_id.is_empty() && agent_out.session_id != "unknown" {
        Some(agent_out.session_id.clone())
    } else {
        None
    };

    // Try validation
    if validate_json_output(&result_text, schema).is_ok() {
        println!("{}", result_text);
        return Ok(());
    }

    // Validation failed — collect errors for retry/reporting
    let initial_errors = validate_json_output(&result_text, schema).unwrap_err();
    debug!("JSON validation failed: {:?}", initial_errors);

    let Some(sid) = session_id else {
        bail!("JSON validation failed:\n- {}", initial_errors.join("\n- "));
    };

    // Try to retry via session resume
    let mut last_errors = initial_errors;
    for attempt in 1..=MAX_JSON_RETRIES {
        debug!("JSON retry attempt {}/{}", attempt, MAX_JSON_RETRIES);

        let correction_prompt = build_correction_prompt(&last_errors);

        match agent.run_resume_with_prompt(&sid, &correction_prompt).await {
            Ok(Some(retry_output)) => {
                if let Some(retry_text) = retry_output.final_result() {
                    if validate_json_output(retry_text, schema).is_ok() {
                        println!("{}", retry_text);
                        return Ok(());
                    }
                    last_errors = validate_json_output(retry_text, schema).unwrap_err();
                } else {
                    last_errors = vec!["Agent returned no result text".to_string()];
                }
            }
            Ok(None) => {
                last_errors = vec!["Agent produced no output on retry".to_string()];
            }
            Err(e) => {
                debug!("Resume with prompt failed: {}", e);
                break;
            }
        }
    }

    bail!(
        "JSON validation failed after {} retries. Last errors:\n- {}",
        MAX_JSON_RETRIES,
        last_errors.join("\n- ")
    )
}

/// Validate JSON output, optionally against a schema.
fn validate_json_output(text: &str, schema: &Option<serde_json::Value>) -> Result<(), Vec<String>> {
    if let Some(schema) = schema {
        json_validation::validate_json_schema(text, schema)?;
    } else {
        json_validation::validate_json(text).map_err(|e| vec![e])?;
    }
    Ok(())
}

/// Build a correction prompt for retrying invalid JSON.
fn build_correction_prompt(errors: &[String]) -> String {
    let error_list: String = errors
        .iter()
        .map(|e| format!("- {}", e))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Your previous response was not valid JSON. Errors:\n{}\n\nPlease respond with ONLY valid JSON. No markdown fences, no explanations.",
        error_list
    )
}

/// Process and display structured agent output
fn process_agent_output(
    output: &crate::output::AgentOutput,
    show_usage: bool,
    verbose: bool,
) -> Result<()> {
    use crate::output::{Event, LogLevel};

    // Show decorations only when verbose is enabled (or not in quiet mode for non-exec paths)
    let show_decorations = verbose && !logging::is_quiet();

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

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
