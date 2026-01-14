mod agent;
mod claude;
mod codex;
mod config;
mod copilot;
mod factory;
mod gemini;
mod logging;

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
    },
    /// Run the Copilot agent
    Copilot {
        /// The prompt to send to the agent (optional in interactive mode, required with -n)
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

        /// Run in non-interactive mode (process prompt and exit, requires a prompt)
        #[arg(short = 'n', long = "non-interactive")]
        non_interactive: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    logging::init(cli.debug);
    debug!("Debug logging enabled");

    match cli.command {
        Commands::Codex {
            prompt,
            system_prompt,
            model,
            root,
            auto_approve,
            print,
        } => {
            run_agent(
                "Codex",
                system_prompt,
                model,
                root,
                auto_approve,
                prompt,
                print,
                false,
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
        } => {
            run_agent(
                "Claude",
                system_prompt,
                model,
                root,
                auto_approve,
                prompt,
                print,
                false,
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
        } => {
            run_agent(
                "Gemini",
                system_prompt,
                model,
                root,
                auto_approve,
                prompt,
                print,
                false,
            )
            .await?;
        }
        Commands::Copilot {
            prompt,
            system_prompt,
            model,
            root,
            auto_approve,
            non_interactive,
        } => {
            if non_interactive && prompt.is_none() {
                bail!("Non-interactive mode requires a prompt");
            }

            run_agent(
                "Copilot",
                system_prompt,
                model,
                root,
                auto_approve,
                prompt,
                false,
                non_interactive,
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
    non_interactive: bool,
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

    // Create agent with spinner
    let spinner = logging::spinner(format!("Initializing {} agent", agent_name));
    let agent = AgentFactory::create(
        &agent_name_lower,
        system_prompt,
        model.clone(),
        root,
        auto_approve,
    )?;
    logging::finish_spinner_quiet(&spinner);

    // Log agent creation details after spinner clears
    debug!("Agent configuration complete");

    // Get the actual model being used (after resolution)
    let model_name = agent.get_model();
    let auto_approve_suffix = if auto_approve { " (auto approve)" } else { "" };
    println!("\x1b[32m✓\x1b[0m {} initialized with model {}{}", agent_name, model_name, auto_approve_suffix);

    // Run the agent
    let mode = if print || non_interactive {
        "non-interactive"
    } else {
        "interactive"
    };
    info!("Starting {} session", mode);

    if print || non_interactive {
        agent.run(prompt.as_deref()).await?;
    } else {
        agent.run_interactive(prompt.as_deref()).await?;
    }

    // Cleanup
    debug!("Cleaning up agent resources");
    agent.cleanup().await?;
    info!("Session terminated");

    Ok(())
}
