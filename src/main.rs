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
mod session_log;
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
        #[arg(conflicts_with = "resume", conflicts_with = "continue_session")]
        prompt: Option<String>,

        /// Resume a specific session
        #[arg(long, value_name = "SESSION_ID", conflicts_with = "continue_session")]
        resume: Option<String>,

        /// Resume the most recent tracked session
        #[arg(long = "continue")]
        continue_session: bool,
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
    /// Historical session log utilities
    Logs {
        #[command(subcommand)]
        command: LogsCommand,
    },
    /// Show manual pages for commands
    Man {
        /// Command to show help for (run, exec, review, config, man)
        command: Option<String>,
    },
}

#[derive(Subcommand)]
enum LogsCommand {
    /// Import historical provider logs into the unified session log store
    Import,
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
            Commands::Review { .. } => bail!("--json-stream cannot be used with review"),
            Commands::Config { .. } => bail!("--json-stream cannot be used with config"),
            Commands::Logs { .. } => bail!("--json-stream cannot be used with logs"),
            Commands::Run {
                prompt: _,
                resume,
                continue_session,
            } if resume.is_some() || *continue_session => {
                bail!("--json-stream cannot be used with run --resume or run --continue")
            }
            Commands::Run { prompt, .. } if prompt.is_none() => {
                bail!("--json-stream requires a prompt (use exec or run with a prompt)")
            }
            _ => {}
        }
    }

    // Validate --json/--json-schema usage and parse schema once
    let json_schema: Option<serde_json::Value> = if json_mode {
        match &cli.command {
            Commands::Review { .. } => bail!("--json/--json-schema cannot be used with review"),
            Commands::Config { .. } => bail!("--json/--json-schema cannot be used with config"),
            Commands::Logs { .. } => bail!("--json/--json-schema cannot be used with logs"),
            Commands::Run {
                resume,
                continue_session,
                ..
            } if resume.is_some() || *continue_session => {
                bail!("--json/--json-schema cannot be used with run --resume or run --continue")
            }
            Commands::Run { prompt, .. } if prompt.is_none() => {
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
            Commands::Logs { .. } => bail!("--worktree cannot be used with logs"),
            Commands::Run {
                resume,
                continue_session,
                ..
            } if resume.is_some() || *continue_session => {
                bail!("--worktree cannot be used with run --resume or run --continue")
            }
            _ => {}
        }
    }

    // Validate --sandbox usage
    if cli.sandbox.is_some() {
        match &cli.command {
            Commands::Review { .. } => bail!("--sandbox cannot be used with review"),
            Commands::Config { .. } => bail!("--sandbox cannot be used with config"),
            Commands::Logs { .. } => bail!("--sandbox cannot be used with logs"),
            Commands::Man { .. } => bail!("--sandbox cannot be used with man"),
            Commands::Run {
                resume,
                continue_session,
                ..
            } if resume.is_some() || *continue_session => {
                bail!("--sandbox cannot be used with run --resume or run --continue")
            }
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
            Commands::Review { .. } => bail!("auto cannot be used with review"),
            Commands::Config { .. } => bail!("auto cannot be used with config"),
            Commands::Logs { .. } => bail!("auto cannot be used with logs"),
            Commands::Run {
                resume,
                continue_session,
                ..
            } if resume.is_some() || *continue_session => {
                bail!("auto cannot be used with run --resume or run --continue")
            }
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
        Commands::Logs { command } => {
            debug!("Running logs subcommand: {:?}", std::mem::discriminant(&command));
            run_logs(command, cli.root.as_deref())?;
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
                provider_explicit: cli.provider.is_some(),
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
    provider_explicit: bool,
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
const MAN_REVIEW: &str = include_str!("../man/review.md");
const MAN_CONFIG: &str = include_str!("../man/config.md");
const MAN_LOGS: &str = include_str!("../man/logs.md");
const MAN_MAN: &str = include_str!("../man/man.md");

/// AI-oriented reference document for `--help-agent`.
const HELP_AGENT: &str = include_str!("../man/help-agent.md");

/// Print a manpage to stdout.
fn print_manpage(command: Option<&str>) -> Result<()> {
    let content = match command {
        None | Some("agent") => MAN_AGENT,
        Some("run") => MAN_RUN,
        Some("exec") => MAN_EXEC,
        Some("review") => MAN_REVIEW,
        Some("config") => MAN_CONFIG,
        Some("logs") => MAN_LOGS,
        Some("man") => MAN_MAN,
        Some(other) => bail!(
            "No manual entry for '{}'. Available: run, exec, review, config, logs, man",
            other
        ),
    };
    print!("{}", content);
    Ok(())
}

fn run_resume_id(action: &Commands) -> Option<&str> {
    match action {
        Commands::Run { resume, .. } => resume.as_deref(),
        _ => None,
    }
}

fn run_logs(command: LogsCommand, root: Option<&str>) -> Result<()> {
    match command {
        LogsCommand::Import => {
            let imported = crate::session_log::run_default_backfill(root)?;
            println!("Imported {} historical session log(s)", imported);
        }
    }
    Ok(())
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

fn is_resume_run(action: &Commands) -> bool {
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

struct PlainSessionSetup {
    session_id: Option<String>,
    workspace_path: Option<String>,
}

#[derive(Clone)]
struct ResumeTarget {
    entry: session::SessionEntry,
    matched_by_wrapper_id: bool,
}

struct DiscoveredSession {
    provider: String,
    provider_session_id: String,
    workspace_path: Option<String>,
    discovery_source: String,
}

fn current_workspace(root: Option<&str>) -> String {
    if let Some(root) = root {
        root.to_string()
    } else if let Ok(repo_root) = worktree::git_repo_root(None) {
        repo_root.to_string_lossy().to_string()
    } else {
        std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    }
}

fn wrapper_worktrees_root() -> Option<std::path::PathBuf> {
    home_dir().map(|home| home.join(".agent").join("worktrees"))
}

fn is_wrapper_worktree_path(path: &str) -> bool {
    let Some(root) = wrapper_worktrees_root() else {
        return false;
    };
    std::path::Path::new(path).starts_with(root)
}

fn worktree_name_from_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Set up worktree session state: generate IDs, create worktree.
/// All providers get the same treatment — worktree at `~/.agent/worktrees/<project>/<name>`.
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
) -> PlainSessionSetup {
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
        if let Some(session_id) = p.session_id {
            claude_agent.set_session_id(session_id);
        }
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

fn print_resume_hint(wrapper_session_id: &str, provider_session_id: Option<&str>, label: &str) {
    println!(
        "\x1b[32m✓\x1b[0m {} kept. Resume with: agent run --resume {}",
        label, wrapper_session_id
    );
    if let Some(provider_session_id) = provider_session_id
        && provider_session_id != wrapper_session_id
    {
        println!(
            "\x1b[32m✓\x1b[0m Native provider ID: {}",
            provider_session_id
        );
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

fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(std::path::PathBuf::from)
}

fn detect_provider_session(session_id: &str) -> Option<DiscoveredSession> {
    let home = home_dir()?;

    let claude_projects = home.join(".claude/projects");
    if let Ok(projects) = std::fs::read_dir(&claude_projects) {
        for project in projects.flatten() {
            let candidate = project.path().join(format!("{}.jsonl", session_id));
            if candidate.exists() {
                let workspace_path = std::fs::read_to_string(&candidate)
                    .ok()
                    .and_then(|content| {
                        content.lines().find_map(|line| {
                            serde_json::from_str::<serde_json::Value>(line)
                                .ok()
                                .and_then(|json| {
                                    json.get("cwd")
                                        .and_then(|value| value.as_str())
                                        .map(str::to_string)
                                })
                        })
                    });
                return Some(DiscoveredSession {
                    provider: "claude".to_string(),
                    provider_session_id: session_id.to_string(),
                    workspace_path,
                    discovery_source: candidate.to_string_lossy().to_string(),
                });
            }
        }
    }

    let codex_history = home.join(".codex/history.jsonl");
    if let Ok(content) = std::fs::read_to_string(&codex_history) {
        let needle = format!("\"session_id\":\"{}\"", session_id);
        if content.contains(&needle) {
            return Some(DiscoveredSession {
                provider: "codex".to_string(),
                provider_session_id: session_id.to_string(),
                workspace_path: None,
                discovery_source: codex_history.to_string_lossy().to_string(),
            });
        }
    }

    let gemini_tmp = home.join(".gemini/tmp");
    if let Ok(projects) = std::fs::read_dir(&gemini_tmp) {
        for project in projects.flatten() {
            let chats = project.path().join("chats");
            if let Ok(files) = std::fs::read_dir(&chats) {
                for file in files.flatten() {
                    if let Ok(content) = std::fs::read_to_string(file.path()) {
                        let needle = format!("\"sessionId\": \"{}\"", session_id);
                        if content.contains(&needle) {
                            return Some(DiscoveredSession {
                                provider: "gemini".to_string(),
                                provider_session_id: session_id.to_string(),
                                workspace_path: None,
                                discovery_source: file.path().to_string_lossy().to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    let copilot_dir = home.join(".copilot/session-state").join(session_id);
    if copilot_dir.join("events.jsonl").exists() {
        return Some(DiscoveredSession {
            provider: "copilot".to_string(),
            provider_session_id: session_id.to_string(),
            workspace_path: None,
            discovery_source: copilot_dir.to_string_lossy().to_string(),
        });
    }

    None
}

fn cache_discovered_session(
    discovered: &DiscoveredSession,
    root: Option<&str>,
) -> session::SessionEntry {
    let workspace_path = discovered
        .workspace_path
        .clone()
        .unwrap_or_else(|| current_workspace(root));
    let is_worktree = is_wrapper_worktree_path(&workspace_path);
    let entry = session::SessionEntry {
        session_id: discovered.provider_session_id.clone(),
        provider: discovered.provider.clone(),
        model: String::new(),
        worktree_path: workspace_path.clone(),
        worktree_name: if is_worktree {
            worktree_name_from_path(&workspace_path)
        } else {
            String::new()
        },
        created_at: chrono::Utc::now().to_rfc3339(),
        provider_session_id: Some(discovered.provider_session_id.clone()),
        sandbox_name: None,
        is_worktree,
        discovered: true,
        discovery_source: Some(discovered.discovery_source.clone()),
        log_path: None,
        log_completeness: "partial".to_string(),
    };

    let mut store = session::SessionStore::load(root).unwrap_or_default();
    store.add(entry.clone());
    if let Err(e) = store.save(root) {
        log::warn!("Failed to cache discovered session: {}", e);
    }

    entry
}

fn resolve_resume_target(requested_id: &str, root: Option<&str>) -> Option<ResumeTarget> {
    let store = session::SessionStore::load(root).unwrap_or_default();
    if let Some(entry) = store.find_by_any_id(requested_id) {
        return Some(ResumeTarget {
            entry: entry.clone(),
            matched_by_wrapper_id: store.find_by_session_id(requested_id).is_some(),
        });
    }

    let discovered = detect_provider_session(requested_id)?;
    let entry = cache_discovered_session(&discovered, root);
    Some(ResumeTarget {
        entry,
        matched_by_wrapper_id: false,
    })
}

fn resolve_continue_target(root: Option<&str>) -> Option<ResumeTarget> {
    let store = session::SessionStore::load(root).unwrap_or_default();
    store.latest().map(|entry| ResumeTarget {
        entry: entry.clone(),
        matched_by_wrapper_id: true,
    })
}

fn discover_provider_session_id(
    provider: &str,
    wrapper_session_id: Option<&str>,
    _root: Option<&str>,
    _wt: &WorktreeSetup,
    _plain: &PlainSessionSetup,
) -> Option<String> {
    match provider {
        "claude" => wrapper_session_id.map(str::to_string),
        "codex" => {
            let history = home_dir()?.join(".codex/history.jsonl");
            let content = std::fs::read_to_string(history).ok()?;
            content
                .lines()
                .rev()
                .find_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
                .and_then(|json| {
                    json.get("session_id")
                        .and_then(|value| value.as_str())
                        .map(str::to_string)
                })
        }
        "gemini" => {
            let gemini_tmp = home_dir()?.join(".gemini/tmp");
            let mut newest: Option<(std::time::SystemTime, String)> = None;
            let projects = std::fs::read_dir(gemini_tmp).ok()?;
            for project in projects.flatten() {
                let chats = project.path().join("chats");
                let files = match std::fs::read_dir(chats) {
                    Ok(files) => files,
                    Err(_) => continue,
                };
                for file in files.flatten() {
                    let path = file.path();
                    let metadata = match file.metadata() {
                        Ok(metadata) => metadata,
                        Err(_) => continue,
                    };
                    let modified = match metadata.modified() {
                        Ok(modified) => modified,
                        Err(_) => continue,
                    };
                    let content = match std::fs::read_to_string(path) {
                        Ok(content) => content,
                        Err(_) => continue,
                    };
                    let session_id = match serde_json::from_str::<serde_json::Value>(&content)
                        .ok()
                        .and_then(|json| {
                            json.get("sessionId")
                                .and_then(|value| value.as_str())
                                .map(str::to_string)
                        }) {
                        Some(session_id) => session_id,
                        None => continue,
                    };
                    if newest
                        .as_ref()
                        .map(|(current, _)| modified > *current)
                        .unwrap_or(true)
                    {
                        newest = Some((modified, session_id));
                    }
                }
            }
            newest.map(|(_, session_id)| session_id)
        }
        "copilot" => {
            let chat_sessions = home_dir()?.join(".copilot/session-state");
            let mut newest: Option<(std::time::SystemTime, String)> = None;
            let entries = std::fs::read_dir(chat_sessions).ok()?;
            for entry in entries.flatten() {
                let events_path = entry.path().join("events.jsonl");
                if !events_path.exists() {
                    continue;
                }
                let metadata = match std::fs::metadata(&events_path) {
                    Ok(metadata) => metadata,
                    Err(_) => continue,
                };
                let modified = match metadata.modified() {
                    Ok(modified) => modified,
                    Err(_) => continue,
                };
                let session_id = entry.file_name().to_string_lossy().to_string();
                if newest
                    .as_ref()
                    .map(|(current, _)| modified > *current)
                    .unwrap_or(true)
                {
                    newest = Some((modified, session_id));
                }
            }
            newest.map(|(_, session_id)| session_id)
        }
        _ => None,
    }
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
        } => {
            if resume.is_some() || continue_session {
                info!("Resuming session");
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
        Commands::Logs { .. } => "logs",
        Commands::Man { .. } => "man",
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
    if let Some(entry) = store.sessions.iter_mut().find(|entry| entry.session_id == session_id) {
        entry.log_path = log_path;
        entry.log_completeness = completeness.to_string();
        let _ = store.save(root);
    }
}

async fn run_agent_action(mut params: AgentActionParams) -> Result<()> {
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
    } = params;

    let is_exec = matches!(action, Commands::Exec { .. });
    let show_wrapper = !quiet && (!is_exec || verbose);

    let system_prompt =
        augment_system_prompt_for_json(system_prompt, json_mode, &provider, &json_schema);
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
            model = Some(target.entry.model.clone());
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

    let plain = setup_plain_session(&action, json_mode, &root);
    let wrapper_session_id = plain.session_id.clone();
    let log_session_id = wrapper_session_id
        .clone()
        .or_else(|| resume_target.as_ref().map(|target| target.entry.session_id.clone()))
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
            agent_name: capitalize(&provider),
            system_prompt,
            model,
            effective_root: effective_root.clone(),
            session_id: wrapper_session_id.clone(),
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
            capitalize(&provider),
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
    };
    let live_adapter = crate::session_log::live_adapter_for_provider(
        &provider,
        live_ctx,
        should_enable_live_session_logs(&action, json_mode),
    );
    let log_coordinator =
        crate::session_log::SessionLogCoordinator::start(root.as_deref(), log_metadata, live_adapter)?;
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

    let exec_ctx = ExecutionContext {
        provider: &provider,
        json_mode,
        json_schema: &json_schema,
        output_fmt: output_fmt_clone.as_deref(),
        show_usage,
        verbose,
    };
    let action_result = execute_action(action, &mut *agent, &exec_ctx, Some(log_coordinator.writer())).await;
    if let Err(err) = &action_result {
        log_coordinator
            .finish(false, Some(err.to_string()))
            .await?;
        return Err(anyhow::anyhow!(err.to_string()));
    }

    let wrapper_session_id = wt
        .session_id
        .as_deref()
        .or(sb.session_id.as_deref())
        .or(plain.session_id.as_deref());
    let native_session_id =
        discover_provider_session_id(&provider, wrapper_session_id, root.as_deref(), &wt, &plain);
    if let Some(native_session_id) = &native_session_id {
        log_coordinator
            .writer()
            .set_provider_session_id(Some(native_session_id.clone()))?;
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
                let store = session::SessionStore::load(root.as_deref()).unwrap_or_default();
                let provider_session_id = store
                    .find_by_session_id(&sid)
                    .and_then(|entry| entry.provider_session_id.as_deref());
                print_resume_hint(&sid, provider_session_id, "Workspace");
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
        let store = session::SessionStore::load(root).unwrap_or_default();
        let provider_session_id = store
            .find_by_session_id(session_id)
            .and_then(|entry| entry.provider_session_id.as_deref());
        print_resume_hint(session_id, provider_session_id, "Sandbox");
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
        let store = session::SessionStore::load(root).unwrap_or_default();
        let provider_session_id = store
            .find_by_session_id(session_id)
            .and_then(|entry| entry.provider_session_id.as_deref());
        print_resume_hint(session_id, provider_session_id, "Workspace");
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
        AgentFactory::create("codex", system_prompt, model, root.clone(), auto_approve, add_dirs)?;
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
    let workspace_path = root.clone().or_else(|| std::env::current_dir().ok().map(|p| p.to_string_lossy().to_string()));
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
        },
        true,
    );
    let log_coordinator =
        crate::session_log::SessionLogCoordinator::start(root.as_deref(), log_metadata, live_adapter)?;
    let review_prompt = format!(
        "review uncommitted={} base={:?} commit={:?} title={:?}",
        uncommitted, base, commit, title
    );
    crate::session_log::record_prompt(log_coordinator.writer(), Some(&review_prompt))?;

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
            log_coordinator
                .finish(false, Some(err.to_string()))
                .await?;
            Err(err)
        }
    }?;

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
