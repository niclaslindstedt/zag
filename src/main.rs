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

    /// Show token usage statistics (only applies to JSON output mode)
    #[arg(long, global = true)]
    show_usage: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the Codex agent
    Codex {
        /// The prompt to send to the agent (optional - starts interactive session if omitted)
        prompt: Option<String>,

        /// System prompt to configure agent behavior
        #[arg(short, long)]
        system_prompt: Option<String>,

        /// Model to use (gpt-5.2-codex, gpt-5.1-codex-max, gpt-5.1-codex-mini, gpt-5.2)
        #[arg(short, long)]
        model: Option<String>,

        /// Root directory to run the agent in
        #[arg(short, long)]
        root: Option<String>,

        /// Auto-approve all actions (skip permission prompts)
        #[arg(short = 'a', long)]
        auto_approve: bool,

        /// Run in non-interactive mode (print output and exit)
        #[arg(short = 'p', long = "print")]
        print: bool,

        /// Output format for print mode (text, json, stream-json)
        #[arg(short = 'o', long)]
        output: Option<String>,
    },
    /// Run the Claude agent
    Claude {
        /// The prompt to send to the agent (optional - starts interactive session if omitted)
        prompt: Option<String>,

        /// System prompt to configure agent behavior
        #[arg(short, long)]
        system_prompt: Option<String>,

        /// Model to use (sonnet, opus, haiku)
        #[arg(short, long)]
        model: Option<String>,

        /// Root directory to run the agent in
        #[arg(short, long)]
        root: Option<String>,

        /// Auto-approve all actions (skip permission prompts)
        #[arg(short = 'a', long)]
        auto_approve: bool,

        /// Run in non-interactive mode (print output and exit)
        #[arg(short = 'p', long = "print")]
        print: bool,

        /// Output format for print mode (text, json, stream-json)
        #[arg(short = 'o', long)]
        output: Option<String>,
    },
    /// Run the Gemini agent
    Gemini {
        /// The prompt to send to the agent (optional - starts interactive session if omitted)
        prompt: Option<String>,

        /// System prompt to configure agent behavior
        #[arg(short, long)]
        system_prompt: Option<String>,

        /// Model to use (auto, gemini-2.5-pro, gemini-2.5-flash, gemini-2.5-flash-lite)
        #[arg(short, long)]
        model: Option<String>,

        /// Root directory to run the agent in
        #[arg(short, long)]
        root: Option<String>,

        /// Auto-approve all actions (skip permission prompts)
        #[arg(short = 'a', long)]
        auto_approve: bool,

        /// Run in non-interactive mode (print output and exit)
        #[arg(short = 'p', long = "print")]
        print: bool,

        /// Output format for print mode (text, json, stream-json)
        #[arg(short = 'o', long)]
        output: Option<String>,
    },
    /// Run the Copilot agent
    Copilot {
        /// The prompt to send to the agent (optional in interactive mode, required with -p)
        prompt: Option<String>,

        /// System prompt to configure agent behavior
        #[arg(short, long)]
        system_prompt: Option<String>,

        /// Model to use (gpt-5, gpt-5.1, gpt-5.2, claude-sonnet-4, gemini-3-pro-preview, etc.)
        #[arg(short, long)]
        model: Option<String>,

        /// Root directory to run the agent in
        #[arg(short, long)]
        root: Option<String>,

        /// Auto-approve all actions (skip permission prompts)
        #[arg(short = 'a', long)]
        auto_approve: bool,

        /// Run in non-interactive mode (print output and exit)
        #[arg(short = 'p', long = "print")]
        print: bool,

        /// Output format for print mode (text, json, stream-json)
        #[arg(short = 'o', long)]
        output: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    logging::init(cli.debug);
    debug!("Debug logging enabled");

    let show_usage = cli.show_usage;

    match cli.command {
        Commands::Codex {
            prompt,
            system_prompt,
            model,
            root,
            auto_approve,
            print,
            output,
        } => {
            run_agent(
                "Codex",
                system_prompt,
                model,
                root,
                auto_approve,
                prompt,
                print,
                output,
                show_usage,
            )
            .await?;
        }
        Commands::Claude {
            prompt,
            system_prompt,
            model,
            root,
            auto_approve,
            print,
            output,
        } => {
            run_agent(
                "Claude",
                system_prompt,
                model,
                root,
                auto_approve,
                prompt,
                print,
                output,
                show_usage,
            )
            .await?;
        }
        Commands::Gemini {
            prompt,
            system_prompt,
            model,
            root,
            auto_approve,
            print,
            output,
        } => {
            run_agent(
                "Gemini",
                system_prompt,
                model,
                root,
                auto_approve,
                prompt,
                print,
                output,
                show_usage,
            )
            .await?;
        }
        Commands::Copilot {
            prompt,
            system_prompt,
            model,
            root,
            auto_approve,
            print,
            output,
        } => {
            if print && prompt.is_none() {
                bail!("Print mode requires a prompt");
            }

            run_agent(
                "Copilot",
                system_prompt,
                model,
                root,
                auto_approve,
                prompt,
                print,
                output,
                show_usage,
            )
            .await?;
        }
    }

    Ok(())
}

async fn run_agent(
    agent_name: &str,
    system_prompt: Option<String>,
    model: Option<String>,
    root: Option<String>,
    auto_approve: bool,
    prompt: Option<String>,
    print: bool,
    output: Option<String>,
    show_usage: bool,
) -> Result<()> {
    let agent_name_lower = agent_name.to_lowercase();

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
    if let Some(ref o) = output {
        debug!("Output format: {}", o);
    }

    // Create agent with spinner
    let spinner = logging::spinner(format!("Initializing {} agent", agent_name));
    let mut agent = AgentFactory::create(
        &agent_name_lower,
        system_prompt,
        model.clone(),
        root,
        auto_approve,
    )?;

    // Set output format if specified
    let output_format = output.clone();
    agent.set_output_format(output);

    logging::finish_spinner_quiet(&spinner);

    // Log agent creation details after spinner clears
    debug!("Agent configuration complete");

    // Get the actual model being used (after resolution)
    let model_name = agent.get_model();
    let auto_approve_suffix = if auto_approve { " (auto approve)" } else { "" };
    println!(
        "\x1b[32m✓\x1b[0m {} initialized with model {}{}",
        agent_name, model_name, auto_approve_suffix
    );

    // Run the agent
    let mode = if print {
        "non-interactive"
    } else {
        "interactive"
    };
    info!("Starting {} session", mode);

    if print {
        let agent_output = agent.run(prompt.as_deref()).await?;

        // Process structured output if available
        if let Some(agent_out) = agent_output {
            // If output format is JSON, print the unified JSON format
            if output_format.as_deref() == Some("json") {
                let json = serde_json::to_string_pretty(&agent_out)?;
                println!("{}", json);
            } else {
                // Otherwise, print the pretty processed output
                process_agent_output(&agent_out, show_usage)?;
            }
        }
        // Note: If agent_output is None, the agent already printed to stdout via Stdio::inherit()
    } else {
        agent.run_interactive(prompt.as_deref()).await?;
    }

    // Cleanup
    debug!("Cleaning up agent resources");
    agent.cleanup().await?;
    info!("Session terminated");

    Ok(())
}

/// Process and display structured agent output
fn process_agent_output(
    output: &crate::output::AgentOutput,
    show_usage: bool,
) -> Result<()> {
    use crate::output::{Event, LogLevel};

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

    // Display final result if available
    if let Some(result) = output.final_result() {
        println!("\n{}", result);
    }

    // Display cost if available
    if let Some(cost) = output.total_cost_usd {
        info!("Total cost: ${:.4}", cost);
    }

    // Display usage statistics if requested
    if show_usage {
        if let Some(usage) = &output.usage {
            info!(
                "Token usage - Input: {}, Output: {}",
                usage.input_tokens, usage.output_tokens
            );

            if let Some(cache_read) = usage.cache_read_tokens {
                if cache_read > 0 {
                    info!("Cache read: {} tokens", cache_read);
                }
            }

            if let Some(cache_creation) = usage.cache_creation_tokens {
                if cache_creation > 0 {
                    info!("Cache created: {} tokens", cache_creation);
                }
            }

            if let Some(web_search) = usage.web_search_requests {
                if web_search > 0 {
                    info!("Web search requests: {}", web_search);
                }
            }

            if let Some(web_fetch) = usage.web_fetch_requests {
                if web_fetch > 0 {
                    info!("Web fetch requests: {}", web_fetch);
                }
            }
        }
    }

    Ok(())
}
