mod agent;
mod claude;
mod codex;
mod config;
mod copilot;
mod gemini;
mod interrupt;
mod pid;
mod process;
mod session;
mod workflow;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use session::{AgentSession, run_sessions};

#[derive(Parser)]
#[command(name = "agent")]
#[command(about = "A wrapper for different AI agents")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum MemoryAction {
    /// Add a new memory to the active workflow (used by agents to remember learnings)
    Add {
        /// The memory content to add
        content: String,

        /// Category to organize memories (e.g., "code_style", "project_structure")
        #[arg(short, long)]
        category: Option<String>,

        /// Workflow name (auto-detected from active workflow if not specified)
        #[arg(long)]
        workflow: Option<String>,
    },
    /// List all memories for a workflow
    List {
        /// Workflow name (auto-detected from active workflow if not specified)
        workflow: Option<String>,

        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,
    },
    /// Search memories by content or category
    Search {
        /// Search query
        query: String,

        /// Workflow name (auto-detected from active workflow if not specified)
        #[arg(long)]
        workflow: Option<String>,
    },
    /// Remove a memory by its ID
    Remove {
        /// Memory ID to remove
        id: usize,

        /// Workflow name (auto-detected from active workflow if not specified)
        #[arg(long)]
        workflow: Option<String>,
    },
    /// Clear all memories for a workflow
    Clear {
        /// Workflow name (required)
        workflow: String,

        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
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
        /// The prompt to send to the agent (optional - starts interactive session if omitted)
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
    },
    /// Signal workflow phase completion and exit (used by agents during interactive sessions)
    Exit,
    /// Manage workflow memories (used by agents to remember learnings across phases)
    Memory {
        #[command(subcommand)]
        action: MemoryAction,
    },
    /// Run a multi-phase workflow
    Workflow {
        /// Workflow name (e.g., "software")
        name: Option<String>,

        /// Resume a previous run instead of starting fresh
        #[arg(short, long)]
        resume: bool,

        /// Checkpoint the current iteration (used by agents during workflows to enable resume)
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

        /// Validate a workflow file
        #[arg(long)]
        validate: Option<String>,

        /// Agent to use (overrides workflow/phase settings)
        #[arg(long)]
        agent: Option<String>,

        /// Auto-approve all actions (skip permission prompts)
        #[arg(short = 'a', long)]
        auto_approve: bool,
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
            let session = AgentSession::new(
                "codex",
                prompt,
                system_prompt,
                model,
                root.clone(),
                auto_approve,
                !print,
            );
            run_sessions(vec![session], root.as_deref()).await?;
        }
        Commands::Claude {
            prompt,
            system_prompt,
            model,
            root,
            auto_approve,
            print,
        } => {
            let session = AgentSession::new(
                "claude",
                prompt,
                system_prompt,
                model,
                root.clone(),
                auto_approve,
                !print,
            );
            run_sessions(vec![session], root.as_deref()).await?;
        }
        Commands::Gemini {
            prompt,
            system_prompt,
            model,
            root,
            auto_approve,
            print,
        } => {
            let session = AgentSession::new(
                "gemini",
                prompt,
                system_prompt,
                model,
                root.clone(),
                auto_approve,
                !print,
            );
            run_sessions(vec![session], root.as_deref()).await?;
        }
        Commands::Copilot {
            prompt,
            system_prompt,
            model,
            root,
            auto_approve,
            print,
        } => {
            let session = AgentSession::new(
                "copilot",
                prompt,
                system_prompt,
                model,
                root.clone(),
                auto_approve,
                !print,
            );
            run_sessions(vec![session], root.as_deref()).await?;
        }
        Commands::Exit => {
            if let Some(session_pid) = pid::read_pid()? {
                pid::write_killed_marker()?;
                kill(Pid::from_raw(session_pid as i32), Signal::SIGTERM)?;
            } else {
                bail!("No active agent session found");
            }
        }
        Commands::Memory { action } => {
            match action {
                MemoryAction::Add {
                    content,
                    category,
                    workflow,
                } => {
                    // Try to get workflow from context if not provided
                    let (workflow_name, root) = match pid::read_workflow_context()? {
                        Some(ctx) => (workflow.unwrap_or(ctx.workflow), ctx.root),
                        None => {
                            let wf = workflow.ok_or_else(|| {
                                anyhow::anyhow!("No active workflow. Provide --workflow name.")
                            })?;
                            (wf, None)
                        }
                    };

                    let manager = workflow::MemoryManager::new(root.as_deref(), &workflow_name);
                    let id = manager.add(content.clone(), category.clone(), None)?;
                    let cat_msg = category.map(|c| format!(" [{}]", c)).unwrap_or_default();
                    println!(
                        "Added memory #{}{} to workflow '{}'",
                        id, cat_msg, workflow_name
                    );
                }
                MemoryAction::List { workflow, category } => {
                    // Try to get workflow from context if not provided
                    let (workflow_name, root) = match pid::read_workflow_context()? {
                        Some(ctx) => (workflow.unwrap_or(ctx.workflow), ctx.root),
                        None => {
                            let wf = workflow.ok_or_else(|| {
                                anyhow::anyhow!("No active workflow. Provide workflow name.")
                            })?;
                            (wf, None)
                        }
                    };

                    let manager = workflow::MemoryManager::new(root.as_deref(), &workflow_name);
                    let memories = manager.list(category.as_deref())?;

                    if memories.is_empty() {
                        println!("No memories for workflow '{}'", workflow_name);
                    } else {
                        println!("Memories for workflow '{}':", workflow_name);
                        for memory in memories {
                            println!("  {}", memory);
                        }
                    }
                }
                MemoryAction::Search { query, workflow } => {
                    // Try to get workflow from context if not provided
                    let (workflow_name, root) = match pid::read_workflow_context()? {
                        Some(ctx) => (workflow.unwrap_or(ctx.workflow), ctx.root),
                        None => {
                            let wf = workflow.ok_or_else(|| {
                                anyhow::anyhow!("No active workflow. Provide --workflow name.")
                            })?;
                            (wf, None)
                        }
                    };

                    let manager = workflow::MemoryManager::new(root.as_deref(), &workflow_name);
                    let results = manager.search(&query)?;

                    if results.is_empty() {
                        println!(
                            "No memories matching '{}' in workflow '{}'",
                            query, workflow_name
                        );
                    } else {
                        println!(
                            "Found {} memories matching '{}' in workflow '{}':",
                            results.len(),
                            query,
                            workflow_name
                        );
                        for entry in results {
                            let cat = entry
                                .category
                                .map(|c| format!(" [{}]", c))
                                .unwrap_or_default();
                            println!("  [{}]{} {}", entry.id, cat, entry.content);
                        }
                    }
                }
                MemoryAction::Remove { id, workflow } => {
                    // Try to get workflow from context if not provided
                    let (workflow_name, root) = match pid::read_workflow_context()? {
                        Some(ctx) => (workflow.unwrap_or(ctx.workflow), ctx.root),
                        None => {
                            let wf = workflow.ok_or_else(|| {
                                anyhow::anyhow!("No active workflow. Provide --workflow name.")
                            })?;
                            (wf, None)
                        }
                    };

                    let manager = workflow::MemoryManager::new(root.as_deref(), &workflow_name);
                    if manager.remove(id)? {
                        println!("Removed memory #{} from workflow '{}'", id, workflow_name);
                    } else {
                        println!("Memory #{} not found in workflow '{}'", id, workflow_name);
                    }
                }
                MemoryAction::Clear { workflow, yes } => {
                    if !yes {
                        print!("Clear all memories for workflow '{}'? [y/N] ", workflow);
                        std::io::Write::flush(&mut std::io::stdout())?;
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input)?;
                        if !input.trim().eq_ignore_ascii_case("y") {
                            println!("Cancelled.");
                            return Ok(());
                        }
                    }

                    let manager = workflow::MemoryManager::new(None, &workflow);
                    manager.clear()?;
                    println!("Cleared all memories for workflow '{}'", workflow);
                }
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
            validate,
            agent,
            auto_approve,
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
                workflow::manage::create_workflow(&workflow_name, create_agent, auto_approve)
                    .await?;
                return Ok(());
            }

            // Modify an existing workflow with AI assistance
            if let Some(workflow_name) = modify {
                let modify_agent = agent.as_deref().unwrap_or("claude");
                workflow::manage::modify_workflow(&workflow_name, modify_agent, auto_approve)
                    .await?;
                return Ok(());
            }

            // Delete a user-defined workflow
            if let Some(workflow_name) = delete {
                workflow::manage::delete_workflow(&workflow_name)?;
                return Ok(());
            }

            // Validate a workflow file
            if let Some(path) = validate {
                workflow::validate::validate_workflow_file(&path)?;
                return Ok(());
            }

            // Checkpoint can auto-detect workflow from context, so handle before name check
            if checkpoint {
                workflow::WorkflowEngine::checkpoint(name.as_deref(), run_id.as_deref())?;
                return Ok(());
            }

            // Name is required for all other operations
            let name = name.ok_or_else(|| {
                anyhow::anyhow!("Workflow name is required. Use --list to see available workflows.")
            })?;

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
                engine
                    .resume(&name, run_id.as_deref(), agent.as_deref())
                    .await?;
            } else {
                engine.run(&name, agent.as_deref()).await?;
            }
        }
    }

    Ok(())
}
