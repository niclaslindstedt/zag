mod agent;
mod claude;
mod codex;
mod config;
mod copilot;
mod factory;
mod gemini;
mod logging;
mod output;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
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

    /// Show token usage statistics (only applies to JSON output mode)
    #[arg(long, global = true)]
    show_usage: bool,

    #[command(subcommand)]
    command: Commands,
}

/// Shared flags for all agent subcommands.
#[derive(Parser, Debug, Clone)]
struct SharedFlags {
    /// System prompt to configure agent behavior
    #[arg(short, long)]
    system_prompt: Option<String>,

    /// Model to use (agent-specific or size alias: small, medium, large)
    #[arg(short, long)]
    model: Option<String>,

    /// Root directory to run the agent in
    #[arg(short, long)]
    root: Option<String>,

    /// Auto-approve all actions (skip permission prompts)
    #[arg(short = 'a', long)]
    auto_approve: bool,

    /// Additional directories to include
    #[arg(long = "add-dir")]
    add_dirs: Vec<String>,
}

#[derive(Subcommand)]
enum AgentAction {
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
}

#[derive(Subcommand)]
enum Commands {
    /// Run the Claude agent
    Claude {
        #[command(subcommand)]
        action: AgentAction,

        #[command(flatten)]
        flags: SharedFlags,
    },
    /// Run the Codex agent
    Codex {
        #[command(subcommand)]
        action: AgentAction,

        #[command(flatten)]
        flags: SharedFlags,
    },
    /// Run the Gemini agent
    Gemini {
        #[command(subcommand)]
        action: AgentAction,

        #[command(flatten)]
        flags: SharedFlags,
    },
    /// Run the Copilot agent
    Copilot {
        #[command(subcommand)]
        action: AgentAction,

        #[command(flatten)]
        flags: SharedFlags,
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

        #[command(flatten)]
        flags: SharedFlags,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    logging::init(cli.debug, cli.quiet);
    debug!("Debug logging enabled");

    let show_usage = cli.show_usage;
    let quiet = cli.quiet;

    match cli.command {
        Commands::Claude { action, flags } => {
            run_agent_action("Claude", action, flags, show_usage, quiet).await?;
        }
        Commands::Codex { action, flags } => {
            run_agent_action("Codex", action, flags, show_usage, quiet).await?;
        }
        Commands::Gemini { action, flags } => {
            run_agent_action("Gemini", action, flags, show_usage, quiet).await?;
        }
        Commands::Copilot { action, flags } => {
            run_agent_action("Copilot", action, flags, show_usage, quiet).await?;
        }
        Commands::Review {
            uncommitted,
            base,
            commit,
            title,
            flags,
        } => {
            run_review(uncommitted, base, commit, title, flags, quiet).await?;
        }
    }

    Ok(())
}

async fn run_agent_action(
    agent_name: &str,
    action: AgentAction,
    flags: SharedFlags,
    show_usage: bool,
    quiet: bool,
) -> Result<()> {
    let agent_name_lower = agent_name.to_lowercase();

    // Log configuration details
    if let Some(ref m) = flags.model {
        debug!("Model specified: {}", m);
    }
    if let Some(ref r) = flags.root {
        debug!("Root directory: {}", r);
    }
    if flags.auto_approve {
        debug!("Auto-approve enabled");
    }
    if let Some(ref sp) = flags.system_prompt {
        debug!("System prompt: {}", sp);
    }
    if !flags.add_dirs.is_empty() {
        debug!("Additional directories: {:?}", flags.add_dirs);
    }

    // Extract output/input format from exec action
    let (output_format, input_format) = match &action {
        AgentAction::Exec {
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

    // Create agent with spinner
    let spinner = logging::spinner(format!("Initializing {} agent", agent_name));
    let mut agent = AgentFactory::create(
        &agent_name_lower,
        flags.system_prompt,
        flags.model,
        flags.root,
        flags.auto_approve,
        flags.add_dirs,
    )?;

    // Set output format if specified
    let output_fmt_clone = output_format.clone();
    agent.set_output_format(output_format);

    // Set input format if specified (Claude only)
    if let Some(input_fmt) = input_format
        && agent_name_lower == "claude"
        && let Some(claude_agent) = agent.as_any_mut().downcast_mut::<crate::claude::Claude>()
    {
        claude_agent.set_input_format(Some(input_fmt));
    }

    logging::finish_spinner_quiet(&spinner);
    debug!("Agent configuration complete");

    // Get the actual model being used (after resolution)
    let model_name = agent.get_model();
    let auto_approve_suffix = if flags.auto_approve {
        " (auto approve)"
    } else {
        ""
    };

    if !quiet {
        println!(
            "\x1b[32m✓\x1b[0m {} initialized with model {}{}",
            agent_name, model_name, auto_approve_suffix
        );
    }

    match action {
        AgentAction::Run { prompt } => {
            info!("Starting interactive session");
            agent.run_interactive(prompt.as_deref()).await?;
        }
        AgentAction::Exec { prompt, .. } => {
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
                        process_agent_output(&agent_out, show_usage)?;
                    }
                }
            }
        }
        AgentAction::Resume { session_id, last } => {
            info!("Resuming session");
            agent
                .run_resume(session_id.as_deref(), last)
                .await?;
        }
    }

    // Cleanup
    debug!("Cleaning up agent resources");
    agent.cleanup().await?;
    info!("Session terminated");

    Ok(())
}

async fn run_review(
    uncommitted: bool,
    base: Option<String>,
    commit: Option<String>,
    title: Option<String>,
    flags: SharedFlags,
    quiet: bool,
) -> Result<()> {
    if !uncommitted && base.is_none() && commit.is_none() {
        bail!("Review requires at least one of: --uncommitted, --base <BRANCH>, --commit <SHA>");
    }

    debug!("Starting code review via Codex");

    let spinner = logging::spinner("Initializing Codex for review".to_string());
    let mut agent = AgentFactory::create(
        "codex",
        flags.system_prompt,
        flags.model,
        flags.root,
        flags.auto_approve,
        flags.add_dirs,
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
fn process_agent_output(output: &crate::output::AgentOutput, show_usage: bool) -> Result<()> {
    use crate::output::{Event, LogLevel};

    // Check if quiet mode is enabled
    let quiet = logging::is_quiet();

    if !quiet {
        // Determine minimum log level based on debug flag
        // For now, we'll use Info level; this can be made configurable via CLI flags
        let min_level = LogLevel::Info;

        // Extract and display log entries
        let log_entries = output.to_log_entries(min_level);
        for entry in log_entries {
            match entry.level {
                LogLevel::Debug => debug!("{}", entry.message),
                LogLevel::Info => info!("{}", entry.message),
                LogLevel::Warn => log::warn!("{}", entry.message),
                LogLevel::Error => log::error!("{}", entry.message),
            }
        }

        // Always display tool executions
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

    // Display final result if available (always shown, even in quiet mode)
    if let Some(result) = output.final_result() {
        if quiet {
            println!("{}", result);
        } else {
            println!("\n{}", result);
        }
    }

    if !quiet {
        // Display cost if available
        if let Some(cost) = output.total_cost_usd {
            info!("Total cost: ${:.4}", cost);
        }

        // Display usage statistics if requested
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
