mod agent;
mod claude;
mod codex;
mod config;
mod copilot;
mod factory;
mod gemini;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use factory::AgentFactory;

#[derive(Parser)]
#[command(name = "agent")]
#[command(about = "A wrapper for different AI agents")]
struct Cli {
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

    match cli.command {
        Commands::Codex {
            prompt,
            system_prompt,
            model,
            root,
            auto_approve,
            print,
        } => {
            let agent = AgentFactory::create("codex", system_prompt, model, root, auto_approve)?;

            if print {
                agent.run(prompt.as_deref()).await?;
            } else {
                agent.run_interactive(prompt.as_deref()).await?;
            }

            agent.cleanup().await?;
            println!("Shutting down session");
        }
        Commands::Claude {
            prompt,
            system_prompt,
            model,
            root,
            auto_approve,
            print,
        } => {
            let agent = AgentFactory::create("claude", system_prompt, model, root, auto_approve)?;

            if print {
                agent.run(prompt.as_deref()).await?;
            } else {
                agent.run_interactive(prompt.as_deref()).await?;
            }

            agent.cleanup().await?;
            println!("Shutting down session");
        }
        Commands::Gemini {
            prompt,
            system_prompt,
            model,
            root,
            auto_approve,
            print,
        } => {
            let agent = AgentFactory::create("gemini", system_prompt, model, root, auto_approve)?;

            if print {
                agent.run(prompt.as_deref()).await?;
            } else {
                agent.run_interactive(prompt.as_deref()).await?;
            }

            agent.cleanup().await?;
            println!("Shutting down session");
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

            let agent = AgentFactory::create("copilot", system_prompt, model, root, auto_approve)?;

            if non_interactive {
                agent.run(prompt.as_deref()).await?;
            } else {
                agent.run_interactive(prompt.as_deref()).await?;
            }

            agent.cleanup().await?;
            println!("Shutting down session");
        }
    }

    Ok(())
}
