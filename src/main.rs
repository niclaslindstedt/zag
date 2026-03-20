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
mod ollama;
mod output;
mod process;
mod sandbox;
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

    /// Run inside a Docker sandbox (optionally specify a name)
    #[arg(long, global = true)]
    sandbox: Option<Option<String>>,

    /// Model parameter size for Ollama (e.g., 0.8b, 2b, 4b, 9b, 27b, 35b, 122b)
    #[arg(long, global = true)]
    size: Option<String>,

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
    /// Show manual pages for commands
    Man {
        /// Command to show help for (run, exec, resume, review, config, man)
        command: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Handle --help-agent before clap parsing so it works without a subcommand.
    if std::env::args().any(|a| a == "--help-agent") {
        print!("{}", HELP_AGENT);
        return Ok(());
    }

    let cli = Cli::parse();

    // In exec mode without --verbose, suppress info-level logging (treat as quiet for the logger)
    let is_exec = matches!(cli.command, Commands::Exec { .. });
    let effective_quiet = cli.quiet || (is_exec && !cli.verbose && !cli.debug);

    // Initialize logging
    logging::init(cli.debug, effective_quiet);
    debug!("Debug logging enabled");

    let show_usage = cli.show_usage;
    let quiet = cli.quiet;
    let verbose = cli.verbose;

    // --json-schema implies --json
    let json_mode = cli.json || cli.json_schema.is_some();
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

    // Validate --json/--json-schema usage and parse schema once
    let json_schema: Option<serde_json::Value> = if json_mode {
        match &cli.command {
            Commands::Resume { .. } => bail!("--json/--json-schema cannot be used with resume"),
            Commands::Review { .. } => bail!("--json/--json-schema cannot be used with review"),
            Commands::Config { .. } => bail!("--json/--json-schema cannot be used with config"),
            Commands::Run { prompt } if prompt.is_none() => {
                bail!("--json/--json-schema requires a prompt (use exec or run with a prompt)")
            }
            _ => {}
        }

        // Parse and validate schema if provided
        if let Some(ref schema_str) = cli.json_schema {
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
            json_validation::validate_schema(&schema_json).map_err(|e| anyhow::anyhow!("{}", e))?;
            debug!(
                "JSON schema loaded: {} bytes",
                serde_json::to_string(&schema_json)
                    .unwrap_or_default()
                    .len()
            );
            Some(schema_json)
        } else {
            None
        }
    } else {
        None
    };

    // Validate --worktree usage (ignored with resume — worktree comes from session mapping)
    if cli.worktree.is_some() {
        match &cli.command {
            Commands::Review { .. } => bail!("--worktree cannot be used with review"),
            Commands::Config { .. } => bail!("--worktree cannot be used with config"),
            _ => {}
        }
    }

    // Validate --sandbox usage
    if cli.sandbox.is_some() {
        match &cli.command {
            Commands::Review { .. } => bail!("--sandbox cannot be used with review"),
            Commands::Config { .. } => bail!("--sandbox cannot be used with config"),
            Commands::Man { .. } => bail!("--sandbox cannot be used with man"),
            _ => {}
        }
        if cli.worktree.is_some() {
            bail!("--sandbox and --worktree are mutually exclusive");
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
        Commands::Man { command } => {
            debug!("Showing manpage for: {:?}", command);
            print_manpage(command.as_deref())?;
        }
        Commands::Config { args } => {
            debug!("Running config subcommand with args: {:?}", args);
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
            debug!("Resolved provider: {}", provider);
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
                sandbox: cli.sandbox,
                size: cli.size,
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
    sandbox: Option<Option<String>>,
    size: Option<String>,
    json_mode: bool,
    json_schema: Option<serde_json::Value>,
    json_stream: bool,
}

const JSON_WRAP_TEMPLATE: &str = include_str!("../prompts/json-wrap/1_0.md");

/// Embedded manpages.
const MAN_AGENT: &str = include_str!("../man/agent.md");
const MAN_RUN: &str = include_str!("../man/run.md");
const MAN_EXEC: &str = include_str!("../man/exec.md");
const MAN_RESUME: &str = include_str!("../man/resume.md");
const MAN_REVIEW: &str = include_str!("../man/review.md");
const MAN_CONFIG: &str = include_str!("../man/config.md");
const MAN_MAN: &str = include_str!("../man/man.md");

/// AI-oriented reference document for `--help-agent`.
const HELP_AGENT: &str = include_str!("../man/help-agent.md");

/// Print a manpage to stdout.
fn print_manpage(command: Option<&str>) -> Result<()> {
    let content = match command {
        None | Some("agent") => MAN_AGENT,
        Some("run") => MAN_RUN,
        Some("exec") => MAN_EXEC,
        Some("resume") => MAN_RESUME,
        Some("review") => MAN_REVIEW,
        Some("config") => MAN_CONFIG,
        Some("man") => MAN_MAN,
        Some(other) => bail!(
            "No manual entry for '{}'. Available: run, exec, resume, review, config, man",
            other
        ),
    };
    print!("{}", content);
    Ok(())
}

/// Wrap a user prompt with explicit JSON instructions for non-Claude agents.
fn wrap_prompt_for_json(prompt: &str) -> String {
    JSON_WRAP_TEMPLATE.replace("{PROMPT}", prompt)
}

/// Handle auto provider/model selection, mutating params in place.
async fn resolve_auto_selection(params: &mut AgentActionParams) -> Result<()> {
    let is_auto_provider = params.provider == "auto";
    let is_auto_model = params.model.as_deref() == Some("auto");

    if !is_auto_provider && !is_auto_model {
        return Ok(());
    }

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

    Ok(())
}

/// Augment the system prompt with JSON instructions for non-Claude agents.
fn augment_system_prompt_for_json(
    system_prompt: Option<String>,
    json_mode: bool,
    provider: &str,
    json_schema: &Option<serde_json::Value>,
) -> Option<String> {
    if !json_mode || provider == "claude" {
        return system_prompt;
    }

    let mut prompt = system_prompt.unwrap_or_default();
    if let Some(schema) = json_schema {
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
}

/// Worktree setup state computed before agent creation.
struct WorktreeSetup {
    is_worktree_session: bool,
    session_id: Option<String>,
    worktree_name: Option<String>,
    effective_root: Option<String>,
    worktree_path: Option<String>,
}

/// Set up worktree session state: generate IDs, create worktree.
/// All providers get the same treatment — worktree at `.git/agent-worktrees/<name>`.
fn setup_worktree(
    worktree_flag: &Option<Option<String>>,
    action: &Commands,
    root: &Option<String>,
    show_wrapper: bool,
) -> Result<WorktreeSetup> {
    let is_worktree_session = worktree_flag.is_some() && !matches!(action, Commands::Resume { .. });

    if !is_worktree_session {
        return Ok(WorktreeSetup {
            is_worktree_session: false,
            session_id: None,
            worktree_name: None,
            effective_root: root.clone(),
            worktree_path: None,
        });
    }

    let session_id = Some(uuid::Uuid::new_v4().to_string());
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
struct SandboxSetup {
    is_sandbox_session: bool,
    sandbox_name: Option<String>,
    session_id: Option<String>,
    workspace: Option<String>,
}

/// Set up sandbox session state: generate name, session ID, determine workspace.
fn setup_sandbox(
    sandbox_flag: &Option<Option<String>>,
    action: &Commands,
    root: &Option<String>,
) -> Result<SandboxSetup> {
    let is_sandbox_session = sandbox_flag.is_some() && !matches!(action, Commands::Resume { .. });

    if !is_sandbox_session {
        return Ok(SandboxSetup {
            is_sandbox_session: false,
            sandbox_name: None,
            session_id: None,
            workspace: None,
        });
    }

    let session_id = Some(uuid::Uuid::new_v4().to_string());
    let sandbox_name = Some(
        sandbox_flag
            .as_ref()
            .unwrap()
            .as_deref()
            .map(String::from)
            .unwrap_or_else(sandbox::generate_name),
    );

    // Determine workspace: root flag > git repo root > current dir
    let workspace = if let Some(r) = root {
        r.clone()
    } else if let Ok(repo_root) = worktree::git_repo_root(None) {
        repo_root.to_string_lossy().to_string()
    } else {
        std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    };

    Ok(SandboxSetup {
        is_sandbox_session: true,
        sandbox_name,
        session_id,
        workspace: Some(workspace),
    })
}

/// Parameters for creating and configuring an agent.
struct AgentSetupParams {
    provider: String,
    agent_name: String,
    system_prompt: Option<String>,
    model: Option<String>,
    effective_root: Option<String>,
    auto_approve: bool,
    add_dirs: Vec<String>,
    output_format: Option<String>,
    input_format: Option<String>,
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
        logging::spinner(format!("Initializing {} agent", p.agent_name))
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
        if let Some(input_fmt) = p.input_format {
            claude_agent.set_input_format(Some(input_fmt));
        }
        if p.json_mode
            && let Some(schema) = json_schema
        {
            let schema_str = serde_json::to_string(schema).unwrap_or_default();
            claude_agent.set_json_schema(Some(schema_str));
        }
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

    logging::finish_spinner_quiet(&spinner);
    debug!("Agent configuration complete");

    Ok((agent, output_fmt_clone))
}

/// Save the session-worktree/sandbox mapping to disk.
fn save_session_mapping(wt: &WorktreeSetup, sb: &SandboxSetup, provider: &str, root: Option<&str>) {
    // Save worktree session mapping
    if let (Some(sid), Some(wt_path), Some(wt_name)) =
        (&wt.session_id, &wt.worktree_path, &wt.worktree_name)
    {
        let mut store = session::SessionStore::load(root).unwrap_or_default();
        store.add(session::SessionEntry {
            session_id: sid.clone(),
            provider: provider.to_string(),
            worktree_path: wt_path.clone(),
            worktree_name: wt_name.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            sandbox_name: None,
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
            worktree_path: workspace.clone(),
            worktree_name: sandbox_name.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            sandbox_name: Some(sandbox_name.clone()),
        });
        if let Err(e) = store.save(root) {
            log::warn!("Failed to save sandbox session mapping: {}", e);
        }
        debug!("Saved sandbox session mapping: {} -> {}", sid, sandbox_name);
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
    root: Option<&'a str>,
}

/// Execute the requested action (run, exec, or resume).
///
/// Returns optional `(session_id, worktree_path)` and optional `(session_id, sandbox_name)` for cleanup.
async fn execute_action(
    action: Commands,
    agent: &mut (dyn crate::agent::Agent + Send + Sync),
    ctx: &ExecutionContext<'_>,
) -> Result<(Option<(String, String)>, Option<(String, String)>)> {
    let mut resume_worktree_info = None;
    let mut resume_sandbox_info = None;

    match action {
        Commands::Run { prompt } => {
            if ctx.json_mode && prompt.is_some() {
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
        Commands::Resume {
            session_id: resume_id,
            last,
        } => {
            debug!("Resume action: session_id={:?}, last={}", resume_id, last);
            if let Some(ref sid) = resume_id {
                let store = session::SessionStore::load(ctx.root).unwrap_or_default();
                if let Some(entry) = store.find_by_session_id(sid) {
                    // Handle sandbox resume
                    if let Some(ref sandbox_name) = entry.sandbox_name {
                        debug!("Resuming in sandbox: {}", sandbox_name);
                        let workspace = if !entry.worktree_path.is_empty() {
                            entry.worktree_path.clone()
                        } else {
                            std::env::current_dir()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string()
                        };
                        let config = sandbox::SandboxConfig {
                            name: sandbox_name.clone(),
                            template: sandbox::template_for_provider(&entry.provider).to_string(),
                            workspace,
                        };
                        agent.set_sandbox(config);
                        resume_sandbox_info = Some((sid.clone(), sandbox_name.clone()));
                    } else {
                        // Handle worktree resume
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
                            let mut store = store;
                            store.remove(sid);
                            let _ = store.save(ctx.root);
                        }
                    }
                }
            }

            info!("Resuming session");
            agent.run_resume(resume_id.as_deref(), last).await?;
        }
        _ => unreachable!(),
    }

    Ok((resume_worktree_info, resume_sandbox_info))
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

async fn run_agent_action(mut params: AgentActionParams) -> Result<()> {
    resolve_auto_selection(&mut params).await?;
    log_config_details(&params);

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
        sandbox: sandbox_flag,
        size,
        json_mode,
        json_schema,
        json_stream,
    } = params;

    let is_exec = matches!(action, Commands::Exec { .. });
    let show_wrapper = !quiet && (!is_exec || verbose);

    let system_prompt =
        augment_system_prompt_for_json(system_prompt, json_mode, &provider, &json_schema);
    if let Some(ref sp) = system_prompt {
        debug!("Effective system prompt: {}", sp);
    }

    let wt = setup_worktree(&worktree_flag, &action, &root, show_wrapper)?;
    let sb = setup_sandbox(&sandbox_flag, &action, &root)?;

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

    let (mut agent, output_fmt_clone) = create_and_configure_agent(
        AgentSetupParams {
            provider: provider.clone(),
            agent_name: agent_name.clone(),
            system_prompt,
            model,
            effective_root: wt.effective_root.clone(),
            auto_approve,
            add_dirs,
            output_format,
            input_format,
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
    let auto_approve_suffix = if auto_approve { " (auto approve)" } else { "" };
    if show_wrapper {
        println!(
            "\x1b[32m✓\x1b[0m {} initialized with model {}{}",
            agent_name, model_display, auto_approve_suffix
        );
    }

    // Save session-worktree mapping before execution (so it survives Ctrl+C)
    save_session_mapping(&wt, &sb, &provider, root.as_deref());

    let is_worktree_session = wt.is_worktree_session;
    let is_interactive_worktree = wt.is_worktree_session && matches!(action, Commands::Run { .. });
    let is_interactive_sandbox = sb.is_sandbox_session && matches!(action, Commands::Run { .. });

    let exec_ctx = ExecutionContext {
        provider: &provider,
        json_mode,
        json_schema: &json_schema,
        output_fmt: output_fmt_clone.as_deref(),
        show_usage,
        verbose,
        root: root.as_deref(),
    };
    let (resume_worktree_info, resume_sandbox_info) =
        execute_action(action, &mut *agent, &exec_ctx).await?;

    // Cleanup
    debug!("Cleaning up agent resources");
    agent.cleanup().await?;
    info!("Session terminated");

    // Sandbox cleanup prompt
    if is_interactive_sandbox {
        if let Some(ref name) = sb.sandbox_name {
            prompt_sandbox_cleanup(
                sb.session_id.as_deref().unwrap_or(""),
                name,
                root.as_deref(),
            )?;
        }
    } else if let Some((ref sid, ref sandbox_name)) = resume_sandbox_info {
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
    } else {
        resume_worktree_info
    };

    if let Some((sid, wtp)) = cleanup_info {
        let wt_path = std::path::Path::new(&wtp);
        let has_changes = wt_path.exists() && worktree::has_changes(wt_path).unwrap_or(true);

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
                println!(
                    "\x1b[32m✓\x1b[0m Workspace kept. Resume with: agent resume {}",
                    sid
                );
            }
        }
    }

    Ok(())
}

/// Prompt the user whether to keep or remove a sandbox after an interactive session.
fn prompt_sandbox_cleanup(session_id: &str, sandbox_name: &str, root: Option<&str>) -> Result<()> {
    use std::io::{self, BufRead, Write};

    debug!(
        "Prompting sandbox cleanup: session={}, sandbox={}",
        session_id, sandbox_name
    );
    println!("\n\x1b[33m>\x1b[0m Sandbox: {}", sandbox_name);
    print!("\x1b[33m>\x1b[0m Keep sandbox? [Y/n] ");
    io::stdout().flush()?;

    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    let answer = line.trim().to_lowercase();

    if answer == "n" || answer == "no" {
        match sandbox::remove_sandbox(sandbox_name) {
            Ok(()) => {
                println!("\x1b[32m✓\x1b[0m Sandbox removed");
            }
            Err(e) => {
                log::warn!("Failed to remove sandbox: {}", e);
                println!("\x1b[31m✗\x1b[0m Failed to remove sandbox: {}", e);
            }
        }
        // Remove session mapping
        let mut store = session::SessionStore::load(root).unwrap_or_default();
        store.remove(session_id);
        let _ = store.save(root);
    } else {
        println!(
            "\x1b[32m✓\x1b[0m Sandbox kept. Resume with: agent resume {}",
            session_id
        );
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

    debug!(
        "Prompting worktree cleanup: session={}, path={}",
        session_id, worktree_path
    );
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

    debug!(
        "Starting code review via Codex (uncommitted={}, base={:?}, commit={:?})",
        uncommitted, base, commit
    );

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

    let raw_result = agent_out
        .final_result()
        .ok_or_else(|| anyhow::anyhow!("Agent output has no result text for JSON validation"))?;
    debug!(
        "JSON mode: raw agent result ({} bytes): {}",
        raw_result.len(),
        raw_result
    );

    let result_text = json_validation::strip_markdown_fences(raw_result).to_string();
    debug!(
        "JSON mode: after fence stripping ({} bytes): {}",
        result_text.len(),
        result_text
    );

    let session_id = if !agent_out.session_id.is_empty() && agent_out.session_id != "unknown" {
        Some(agent_out.session_id.clone())
    } else {
        None
    };

    // Try validation
    if validate_json_output(&result_text, schema).is_ok() {
        // Minify JSON output
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result_text) {
            println!("{}", serde_json::to_string(&parsed)?);
        } else {
            println!("{}", result_text);
        }
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
        debug!("JSON retry correction prompt: {}", correction_prompt);

        match agent.run_resume_with_prompt(&sid, &correction_prompt).await {
            Ok(Some(retry_output)) => {
                if let Some(raw_retry_text) = retry_output.final_result() {
                    let retry_text = json_validation::strip_markdown_fences(raw_retry_text);
                    if validate_json_output(retry_text, schema).is_ok() {
                        // Minify JSON output
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(retry_text) {
                            println!("{}", serde_json::to_string(&parsed)?);
                        } else {
                            println!("{}", retry_text);
                        }
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
