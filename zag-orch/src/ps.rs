use anyhow::{Context, Result, bail};
use zag_agent::process_store::{ProcessEntry, ProcessStore};
use zag_agent::session::SessionStore;
use zag_agent::session_log::{LogEventKind, append_event_to_log, logs_dir};

/// If `id` is the literal `"self"`, resolve it from the `ZAG_PROCESS_ID`
/// environment variable. Otherwise return the id unchanged.
fn resolve_process_id(id: &str) -> Result<String> {
    if id == "self" {
        std::env::var("ZAG_PROCESS_ID").map_err(|_| {
            anyhow::anyhow!(
                "Cannot resolve \"self\": ZAG_PROCESS_ID is not set. \
                 Are you running inside a zag session?"
            )
        })
    } else {
        Ok(id.to_string())
    }
}

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
    check_process_alive(entry.pid)
}

#[cfg(unix)]
fn check_process_alive(pid: u32) -> &'static str {
    use nix::sys::signal::kill;
    use nix::unistd::Pid;
    let pid = Pid::from_raw(pid as i32);
    match kill(pid, None) {
        Ok(()) => "running",
        Err(_) => "dead",
    }
}

#[cfg(not(unix))]
fn check_process_alive(_pid: u32) -> &'static str {
    // On Windows, we cannot cheaply check liveness with signals.
    // Return "running" and let callers handle stale entries.
    "running"
}

#[cfg(unix)]
fn send_signal(pid: u32, signal: nix::sys::signal::Signal) -> Result<()> {
    use nix::sys::signal::kill;
    use nix::unistd::Pid;
    let pid = Pid::from_raw(pid as i32);
    kill(pid, signal).map_err(|e| anyhow::anyhow!("Failed to signal process: {e}"))
}

#[cfg(not(unix))]
fn send_signal(pid: u32, _signal_name: &str) -> Result<()> {
    bail!(
        "Process signaling is not supported on Windows. Use taskkill /PID {} instead.",
        pid
    );
}

/// A process entry with resolved live status.
#[derive(Debug, serde::Serialize)]
pub struct ProcessInfo {
    #[serde(flatten)]
    pub entry: serde_json::Value,
    pub live_status: String,
}

/// List processes with resolved live status.
pub fn list_processes(
    running: bool,
    limit: Option<usize>,
    provider: Option<&str>,
) -> Result<Vec<ProcessInfo>> {
    let store = ProcessStore::load()?;
    let mut entries: Vec<&ProcessEntry> = store.list_recent(limit);
    if running {
        entries.retain(|e| resolve_live_status(e) == "running");
    }
    if let Some(p) = provider {
        entries.retain(|e| e.provider == p);
    }
    Ok(entries
        .iter()
        .map(|e| {
            let mut v = serde_json::to_value(e).unwrap_or_default();
            let live = resolve_live_status(e).to_string();
            if let serde_json::Value::Object(ref mut m) = v {
                m.insert(
                    "live_status".to_string(),
                    serde_json::Value::String(live.clone()),
                );
            }
            ProcessInfo {
                entry: v,
                live_status: live,
            }
        })
        .collect())
}

/// Get a single process by ID with live status.
pub fn get_process(id: &str) -> Result<ProcessInfo> {
    let id = resolve_process_id(id)?;
    let store = ProcessStore::load()?;
    match store.find(&id) {
        Some(e) => {
            let live = resolve_live_status(e).to_string();
            let mut v = serde_json::to_value(e)?;
            if let serde_json::Value::Object(ref mut m) = v {
                m.insert(
                    "live_status".to_string(),
                    serde_json::Value::String(live.clone()),
                );
            }
            Ok(ProcessInfo {
                entry: v,
                live_status: live,
            })
        }
        None => bail!("Process not found: {id}"),
    }
}

/// Send a stop signal (SIGHUP) to a process by ID.
pub fn request_stop(id: &str) -> Result<()> {
    let id = resolve_process_id(id)?;
    let entry = ProcessStore::load()?
        .find(&id)
        .ok_or_else(|| anyhow::anyhow!("Process not found: {id}"))?
        .clone();
    let live = resolve_live_status(&entry);
    if live != "running" {
        bail!("Process {id} is not running (status: {live})");
    }
    stop_process(entry.pid)
}

/// Source of a result string passed to `zag ps kill`.
#[derive(Debug, Clone)]
pub enum KillResult {
    /// Inline result string (positional argument).
    Inline(String),
    /// Path to a file whose contents are used as the result.
    File(std::path::PathBuf),
}

impl KillResult {
    /// Read the result string. For `File`, reads and returns the file
    /// contents (trailing whitespace preserved). Empty strings are valid
    /// at this layer; the validation step decides whether to reject.
    pub fn read(&self) -> Result<String> {
        match self {
            Self::Inline(s) => Ok(s.clone()),
            Self::File(path) => std::fs::read_to_string(path)
                .with_context(|| format!("Failed to read result file: {}", path.display())),
        }
    }
}

/// Resolved data needed to perform a kill: the process entry, the
/// associated session entry (if any), and the result string to record.
struct PreparedKill {
    process_id: String,
    process_entry: ProcessEntry,
    session_entry: Option<zag_agent::session::SessionEntry>,
    result_text: Option<String>,
}

/// Resolve the target process, validate the optional result against the
/// session's `--exit` constraints, and return the data needed to send
/// the kill signal. Does NOT mutate any state — the caller decides when
/// to record the result and signal the process.
fn prepare_kill(id: &str, result: Option<KillResult>) -> Result<PreparedKill> {
    let id = resolve_process_id(id)?;
    let process_entry = ProcessStore::load()?
        .find(&id)
        .ok_or_else(|| anyhow::anyhow!("Process not found: {id}"))?
        .clone();
    let live = resolve_live_status(&process_entry);
    if live != "running" {
        bail!("Process {id} is not running (status: {live})");
    }

    let result_text = match &result {
        Some(r) => Some(r.read()?),
        None => None,
    };

    let session_entry = process_entry.session_id.as_deref().and_then(|sid| {
        SessionStore::load(process_entry.root.as_deref())
            .ok()
            .and_then(|store| store.find_by_any_id(sid).cloned())
    });

    if let Some(s) = session_entry.as_ref() {
        let has_constraints =
            s.exit_hint.is_some() || s.exit_json_mode || s.exit_json_schema.is_some();
        if has_constraints {
            let text = result_text.as_deref().unwrap_or("");
            zag_agent::exit_mode::validate_exit_result(
                text,
                s.exit_hint.as_deref(),
                s.exit_json_mode,
                s.exit_json_schema.as_ref(),
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        }
    }

    Ok(PreparedKill {
        process_id: id,
        process_entry,
        session_entry,
        result_text,
    })
}

/// Apply the prepared kill: record the result (if any), update the
/// process store, and SIGTERM the process.
fn apply_kill(prepared: PreparedKill) -> Result<()> {
    let PreparedKill {
        process_id,
        process_entry,
        session_entry,
        result_text,
    } = prepared;

    // Record the session result *before* signaling the process. If the
    // write fails we abort the kill: the agent is still alive and can
    // retry, but a silent kill with a missing result would lose work.
    if let (Some(text), Some(session)) = (result_text.as_ref(), session_entry.as_ref()) {
        record_session_result(session, text, process_entry.root.as_deref()).map_err(|e| {
            anyhow::anyhow!(
                "Failed to record session result: {e}. Kill aborted — the \
                 process is still running. Retry once the underlying I/O \
                 error is resolved."
            )
        })?;
    }

    let mut store = ProcessStore::load()?;
    store.update_status(&process_id, "killed", None);
    store.save()?;
    kill_process(process_entry.pid)
}

/// Send a kill signal (SIGTERM) to a process by ID, optionally capturing
/// a final result.
///
/// When the target session was launched with `--exit`, the result is
/// validated against the launching constraints (`--exit '<hint>'`,
/// `--json`, `--json-schema`) and recorded as a `SessionResult` event in
/// the session log. If validation fails, the kill is **rejected** — the
/// process keeps running so the agent can self-correct.
pub fn request_kill(id: &str, result: Option<KillResult>) -> Result<()> {
    apply_kill(prepare_kill(id, result)?)
}

/// Write a `SessionResult` event into the session log for `session_entry`.
///
/// Prefers the stored `log_path` (when the session was launched through
/// the normal path), otherwise falls back to
/// `<logs_dir>/sessions/<session_id>.jsonl`.
fn record_session_result(
    session_entry: &zag_agent::session::SessionEntry,
    result: &str,
    root: Option<&str>,
) -> Result<()> {
    let log_path = session_entry
        .log_path
        .as_ref()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            logs_dir(root)
                .join("sessions")
                .join(format!("{}.jsonl", session_entry.session_id))
        });
    append_event_to_log(
        &log_path,
        &session_entry.provider,
        &session_entry.session_id,
        session_entry.provider_session_id.as_deref(),
        LogEventKind::SessionResult {
            result: result.to_string(),
        },
    )
}

pub fn run_ps(command: PsCommand, json: bool) -> Result<()> {
    match command {
        PsCommand::List {
            running,
            limit,
            provider,
            children,
        } => {
            let store = ProcessStore::load()?;
            let mut entries: Vec<&ProcessEntry> = store.list_recent(limit);
            if running {
                entries.retain(|e| resolve_live_status(e) == "running");
            }
            if let Some(ref p) = provider {
                entries.retain(|e| e.provider == *p);
            }
            if let Some(ref parent_id) = children {
                entries.retain(|e| {
                    e.parent_session_id.as_deref() == Some(parent_id)
                        || e.parent_process_id.as_deref() == Some(parent_id)
                });
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
            let id = resolve_process_id(&id)?;
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
                    println!("Status:      {live}");
                    println!("Provider:    {}", e.provider);
                    println!("Model:       {}", e.model);
                    println!("Command:     {}", e.command);
                    println!("Started:     {}", e.started_at);
                    if let Some(ref exited) = e.exited_at {
                        println!("Exited:      {exited}");
                    }
                    if let Some(code) = e.exit_code {
                        println!("Exit code:   {code}");
                    }
                    if let Some(ref sid) = e.session_id {
                        println!("Session ID:  {sid}");
                    }
                    if let Some(ref root) = e.root {
                        println!("Root:        {root}");
                    }
                    if let Some(ref prompt) = e.prompt {
                        println!("Prompt:      {prompt}");
                    }
                }
                None => {
                    bail!("Process not found: {id}");
                }
            }
        }
        PsCommand::Stop { id } => {
            let id = resolve_process_id(&id)?;
            let entry = ProcessStore::load()?
                .find(&id)
                .ok_or_else(|| anyhow::anyhow!("Process not found: {id}"))?
                .clone();
            let live = resolve_live_status(&entry);
            if live != "running" {
                bail!("Process {id} is not running (status: {live})");
            }
            println!(
                "\x1b[33m>\x1b[0m Sending stop signal to process {} ({})",
                entry.pid, entry.id
            );
            stop_process(entry.pid)?;
            println!("\x1b[32m✓\x1b[0m Stop signal sent");
        }
        PsCommand::Kill { id, result, file } => {
            let kill_result = match (result, file) {
                (Some(r), None) => Some(KillResult::Inline(r)),
                (None, Some(p)) => Some(KillResult::File(p)),
                (None, None) => None,
                // clap's `conflicts_with` should prevent this case; bail
                // defensively rather than silently picking one.
                (Some(_), Some(_)) => bail!("`<result>` and `--file` are mutually exclusive"),
            };
            let prepared = prepare_kill(&id, kill_result)?;
            println!(
                "\x1b[33m>\x1b[0m Sending kill signal to process {} ({})",
                prepared.process_entry.pid, prepared.process_entry.id
            );
            apply_kill(prepared)?;
            println!("\x1b[32m✓\x1b[0m Process killed");
        }
    }
    Ok(())
}

#[cfg(unix)]
fn stop_process(pid: u32) -> Result<()> {
    send_signal(pid, nix::sys::signal::Signal::SIGHUP)
}

#[cfg(not(unix))]
fn stop_process(pid: u32) -> Result<()> {
    send_signal(pid, "stop")
}

#[cfg(unix)]
fn kill_process(pid: u32) -> Result<()> {
    send_signal(pid, nix::sys::signal::Signal::SIGTERM)
}

#[cfg(not(unix))]
fn kill_process(pid: u32) -> Result<()> {
    send_signal(pid, "kill")
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
        /// Show only child processes of this session or process ID
        #[arg(long)]
        children: Option<String>,
    },
    /// Show details of a specific process
    Show {
        /// Process ID (or "self" to use current process)
        id: String,
    },
    /// Send stop signal to a running process (graceful stop request)
    Stop {
        /// Process ID (or "self" to use current process)
        id: String,
    },
    /// Send kill signal to a running process (forceful termination).
    ///
    /// When the target session was launched with `--exit`, an optional
    /// `result` (or `--file <path>`) is captured as the session's final
    /// output and written to its log. The result is validated against
    /// any `--exit` hint, `--json`, or `--json-schema` constraint set at
    /// launch; if validation fails, the kill is rejected and the process
    /// keeps running so the agent can self-correct.
    Kill {
        /// Process ID (or "self" to use current process)
        id: String,
        /// Final result to record against the session. Required (or
        /// `--file`) when the session was launched with a non-empty
        /// `--exit '<hint>'`.
        #[arg(conflicts_with = "file")]
        result: Option<String>,
        /// Path to a file whose contents should be used as the final
        /// result. Mutually exclusive with the `result` positional.
        #[arg(long, value_name = "PATH", conflicts_with = "result")]
        file: Option<std::path::PathBuf>,
    },
}

#[cfg(test)]
#[path = "ps_tests.rs"]
mod tests;
