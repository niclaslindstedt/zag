mod agent;
mod claude;
mod codex;
mod copilot;
mod gemini;
mod interrupt;
mod pid;
mod process;
mod session;
mod workflow;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use session::{run_sessions, AgentSession};

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
        /// The prompt to send to the agent
        prompt: String,

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
        /// The prompt to send to the agent
        prompt: String,

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
        /// The prompt to send to the agent
        prompt: String,

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
        /// The prompt to send to the agent
        prompt: String,

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
    },
    /// Kill the parent agent session
    Kill,
    /// Run a multi-phase workflow
    Workflow {
        /// Workflow name (e.g., "software")
        name: Option<String>,

        /// Resume a previous run instead of starting fresh
        #[arg(short, long)]
        resume: bool,

        /// Checkpoint the current iteration (mark as complete for resume)
        #[arg(short, long)]
        checkpoint: bool,

        /// Specific run ID to resume/checkpoint (defaults to latest)
        #[arg(long)]
        run_id: Option<String>,

        /// Root directory for the workflow (default: current directory)
        #[arg(long)]
        root: Option<String>,

        /// List available workflows
        #[arg(short, long)]
        list: bool,

        /// List runs for a workflow
        #[arg(long)]
        list_runs: bool,

        /// Create a new workflow with AI assistance
        #[arg(long)]
        create: Option<String>,

        /// Modify an existing workflow with AI assistance
        #[arg(long)]
        modify: Option<String>,

        /// Delete a user-defined workflow
        #[arg(long)]
        delete: Option<String>,

        /// Agent to use (overrides workflow/phase settings)
        #[arg(long)]
        agent: Option<String>,
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
            let session = AgentSession::new("codex", prompt, system_prompt, model, root, auto_approve, !print);
            run_sessions(vec![session]).await?;
        }
        Commands::Claude {
            prompt,
            system_prompt,
            model,
            root,
            auto_approve,
            print,
        } => {
            let session = AgentSession::new("claude", prompt, system_prompt, model, root, auto_approve, !print);
            run_sessions(vec![session]).await?;
        }
        Commands::Gemini {
            prompt,
            system_prompt,
            model,
            root,
            auto_approve,
            print,
        } => {
            let session = AgentSession::new("gemini", prompt, system_prompt, model, root, auto_approve, !print);
            run_sessions(vec![session]).await?;
        }
        Commands::Copilot {
            prompt,
            system_prompt,
            model,
            root,
            auto_approve,
            print,
        } => {
            let session = AgentSession::new("copilot", prompt, system_prompt, model, root, auto_approve, !print);
            run_sessions(vec![session]).await?;
        }
        Commands::Kill => {
            if let Some(session_pid) = pid::read_pid()? {
                pid::write_killed_marker()?;
                kill(Pid::from_raw(session_pid as i32), Signal::SIGTERM)?;
            } else {
                bail!("No active agent session found");
            }
        }
        Commands::Workflow {
            name,
            resume,
            checkpoint,
            run_id,
            root,
            list,
            list_runs,
            create,
            modify,
            delete,
            agent,
        } => {
            interrupt::init();
            let engine = workflow::WorkflowEngine::new(root.as_deref());

            if list {
                let workflows = engine.list_workflows()?;
                println!("Available workflows:");
                for w in workflows {
                    println!("  - {}", w);
                }
                return Ok(());
            }

            // Create a new workflow with AI assistance
            if let Some(workflow_name) = create {
                let create_agent = agent.as_deref().unwrap_or("claude");
                workflow::manage::create_workflow(&workflow_name, create_agent).await?;
                return Ok(());
            }

            // Modify an existing workflow with AI assistance
            if let Some(workflow_name) = modify {
                let modify_agent = agent.as_deref().unwrap_or("claude");
                workflow::manage::modify_workflow(&workflow_name, modify_agent).await?;
                return Ok(());
            }

            // Delete a user-defined workflow
            if let Some(workflow_name) = delete {
                workflow::manage::delete_workflow(&workflow_name)?;
                return Ok(());
            }

            // Checkpoint can auto-detect workflow from context, so handle before name check
            if checkpoint {
                workflow::WorkflowEngine::checkpoint(name.as_deref(), run_id.as_deref())?;
                return Ok(());
            }

            // Name is required for all other operations
            let name = name.ok_or_else(|| anyhow::anyhow!("Workflow name is required. Use --list to see available workflows."))?;

            if list_runs {
                let runs = engine.list_runs(&name)?;
                if runs.is_empty() {
                    println!("No runs found for workflow: {}", name);
                } else {
                    println!("Runs for workflow '{}':", name);
                    for run in runs {
                        println!("  - {}", run);
                    }
                }
                return Ok(());
            }

            if resume {
                engine.resume(&name, run_id.as_deref(), agent.as_deref()).await?;
            } else {
                engine.run(&name, agent.as_deref()).await?;
            }
        }
    }

    Ok(())
}
