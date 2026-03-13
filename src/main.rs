mod agent;
mod auto_selector;
mod claude;
mod codex;
mod config;
mod copilot;
mod factory;
mod gemini;
mod logging;
mod output;
mod process;
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

    // Validate --worktree usage
    if cli.worktree.is_some() {
        match &cli.command {
            Commands::Resume { .. } => bail!("--worktree cannot be used with resume"),
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

    // Handle worktree creation for non-Claude providers
    let effective_root = if let Some(ref wt_name) = worktree_flag {
        if provider != "claude" {
            let repo_root = worktree::git_repo_root(root.as_deref())?;
            let name = wt_name
                .as_deref()
                .map(String::from)
                .unwrap_or_else(worktree::generate_name);
            let wt_path = worktree::create_worktree(&repo_root, &name)?;
            if show_wrapper {
                println!(
                    "\x1b[32m✓\x1b[0m Worktree created at {}",
                    wt_path.display()
                );
            }
            Some(wt_path.to_string_lossy().to_string())
        } else {
            root.clone()
        }
    } else {
        root.clone()
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

    // Set worktree passthrough for Claude (it handles worktrees natively)
    if let Some(ref wt_name) = worktree_flag
        && provider == "claude"
        && let Some(claude_agent) = agent.as_any_mut().downcast_mut::<crate::claude::Claude>()
    {
        claude_agent.set_worktree(wt_name.clone());
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

    match action {
        Commands::Run { prompt } => {
            info!("Starting interactive session");
            agent.run_interactive(prompt.as_deref()).await?;
        }
        Commands::Exec { prompt, .. } => {
            info!("Starting non-interactive session");
            let agent_output = agent.run(Some(&prompt)).await?;

            if let Some(agent_out) = agent_output {
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
        Commands::Resume { session_id, last } => {
            info!("Resuming session");
            agent
                .run_resume(session_id.as_deref(), last)
                .await?;
        }
        _ => unreachable!(),
    }

    // Cleanup
    debug!("Cleaning up agent resources");
    agent.cleanup().await?;
    info!("Session terminated");

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
    let mut agent = AgentFactory::create(
        "codex",
        system_prompt,
        model,
        root,
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

        if show_usage
            && let Some(usage) = &output.usage
        {
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
