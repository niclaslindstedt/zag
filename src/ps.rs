use anyhow::{Result, bail};
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use zag::process_store::{ProcessEntry, ProcessStore};

/// Resolve the live OS status for an entry that is marked "running".
/// Returns "running", "dead", or the stored status unchanged.
pub fn resolve_live_status(entry: &ProcessEntry) -> &'static str {
    if entry.status != "running" {
        return match entry.status.as_str() {
            "exited" => "exited",
            "killed" => "killed",
            _ => "unknown",
        };
    }
    // Signal 0: does not send a signal but checks whether the process exists.
    let pid = Pid::from_raw(entry.pid as i32);
    match kill(pid, None) {
        Ok(()) => "running",
        Err(_) => "dead",
    }
}

pub fn run_ps(command: PsCommand, json: bool) -> Result<()> {
    match command {
        PsCommand::List {
            running,
            limit,
            provider,
        } => {
            let store = ProcessStore::load()?;
            let mut entries: Vec<&ProcessEntry> = store.list_recent(limit);
            if running {
                entries.retain(|e| resolve_live_status(e) == "running");
            }
            if let Some(ref p) = provider {
                entries.retain(|e| e.provider == *p);
            }
            if json {
                let with_live: Vec<serde_json::Value> = entries
                    .iter()
                    .map(|e| {
                        let mut v = serde_json::to_value(e).unwrap_or_default();
                        if let serde_json::Value::Object(ref mut m) = v {
                            m.insert(
                                "live_status".to_string(),
                                serde_json::Value::String(resolve_live_status(e).to_string()),
                            );
                        }
                        v
                    })
                    .collect();
                println!("{}", serde_json::to_string(&with_live)?);
                return Ok(());
            }
            if entries.is_empty() {
                println!("No processes found.");
                return Ok(());
            }
            println!(
                "{:<38} {:<7} {:<8} {:<10} {:<10} {:<7} {:<22} PROMPT",
                "ID", "PID", "STATUS", "PROVIDER", "MODEL", "CMD", "STARTED"
            );
            println!("{}", "-".repeat(130));
            for e in &entries {
                let live = resolve_live_status(e);
                let prompt_display = e
                    .prompt
                    .as_deref()
                    .unwrap_or("")
                    .chars()
                    .take(40)
                    .collect::<String>();
                println!(
                    "{:<38} {:<7} {:<8} {:<10} {:<10} {:<7} {:<22} {}",
                    e.id,
                    e.pid,
                    live,
                    e.provider,
                    e.model,
                    e.command,
                    e.started_at.chars().take(20).collect::<String>(),
                    prompt_display
                );
            }
        }
        PsCommand::Show { id } => {
            let store = ProcessStore::load()?;
            match store.find(&id) {
                Some(e) => {
                    let live = resolve_live_status(e);
                    if json {
                        let mut v = serde_json::to_value(e)?;
                        if let serde_json::Value::Object(ref mut m) = v {
                            m.insert(
                                "live_status".to_string(),
                                serde_json::Value::String(live.to_string()),
                            );
                        }
                        println!("{}", serde_json::to_string(&v)?);
                        return Ok(());
                    }
                    println!("Process ID:  {}", e.id);
                    println!("PID:         {}", e.pid);
                    println!("Status:      {}", live);
                    println!("Provider:    {}", e.provider);
                    println!("Model:       {}", e.model);
                    println!("Command:     {}", e.command);
                    println!("Started:     {}", e.started_at);
                    if let Some(ref exited) = e.exited_at {
                        println!("Exited:      {}", exited);
                    }
                    if let Some(code) = e.exit_code {
                        println!("Exit code:   {}", code);
                    }
                    if let Some(ref sid) = e.session_id {
                        println!("Session ID:  {}", sid);
                    }
                    if let Some(ref root) = e.root {
                        println!("Root:        {}", root);
                    }
                    if let Some(ref prompt) = e.prompt {
                        println!("Prompt:      {}", prompt);
                    }
                }
                None => {
                    bail!("Process not found: {}", id);
                }
            }
        }
        PsCommand::Stop { id } => {
            let entry = ProcessStore::load()?
                .find(&id)
                .ok_or_else(|| anyhow::anyhow!("Process not found: {}", id))?
                .clone();
            let live = resolve_live_status(&entry);
            if live != "running" {
                bail!("Process {} is not running (status: {})", id, live);
            }
            let pid = Pid::from_raw(entry.pid as i32);
            println!(
                "\x1b[33m>\x1b[0m Sending SIGHUP to process {} ({})",
                entry.pid, entry.id
            );
            kill(pid, Signal::SIGHUP)
                .map_err(|e| anyhow::anyhow!("Failed to stop process {}: {}", entry.pid, e))?;
            println!("\x1b[32m✓\x1b[0m Stop signal sent");
        }
        PsCommand::Kill { id } => {
            let mut store = ProcessStore::load()?;
            let entry = store
                .find(&id)
                .ok_or_else(|| anyhow::anyhow!("Process not found: {}", id))?
                .clone();
            let live = resolve_live_status(&entry);
            if live != "running" {
                bail!("Process {} is not running (status: {})", id, live);
            }
            let pid = Pid::from_raw(entry.pid as i32);
            println!(
                "\x1b[33m>\x1b[0m Sending SIGTERM to process {} ({})",
                entry.pid, entry.id
            );
            kill(pid, Signal::SIGTERM)
                .map_err(|e| anyhow::anyhow!("Failed to kill process {}: {}", entry.pid, e))?;
            store.update_status(&id, "killed", None);
            store.save()?;
            println!("\x1b[32m✓\x1b[0m Process killed");
        }
    }
    Ok(())
}

#[derive(clap::Subcommand)]
pub enum PsCommand {
    /// List processes (default)
    List {
        /// Show only running processes
        #[arg(long)]
        running: bool,
        /// Show only the N most recent processes
        #[arg(short = 'n', long)]
        limit: Option<usize>,
        /// Filter by provider
        #[arg(short = 'p', long)]
        provider: Option<String>,
    },
    /// Show details of a specific process
    Show {
        /// Process ID
        id: String,
    },
    /// Send SIGHUP to a running process (graceful stop request)
    Stop {
        /// Process ID
        id: String,
    },
    /// Send SIGTERM to a running process (forceful termination)
    Kill {
        /// Process ID
        id: String,
    },
}
