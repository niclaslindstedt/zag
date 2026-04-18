//! Spawn command: launch an agent session in the background and return the session ID.

use anyhow::Result;
use log::debug;
use std::fs::{self, File};
use std::path::PathBuf;
use zag_agent::config::Config;
use zag_agent::process_store::{ProcessEntry, ProcessStore};
use zag_agent::session::{SessionEntry, SessionStore};

use crate::types::SessionMetadata;
use crate::util::current_workspace;

/// Parameters for the spawn command.
pub struct SpawnParams {
    pub prompt: Option<String>,
    pub provider: String,
    pub model: Option<String>,
    pub root: Option<String>,
    pub auto_approve: bool,
    pub system_prompt: Option<String>,
    pub add_dirs: Vec<String>,
    pub size: Option<String>,
    pub max_turns: Option<u32>,
    pub timeout: Option<String>,
    pub json: bool,
    pub metadata: SessionMetadata,
    pub depends_on: Vec<String>,
    pub inject_context: bool,
    pub retried_from: Option<String>,
    pub interactive: bool,
    /// Extra environment variables to set on the spawned process.
    pub env_vars: Vec<(String, String)>,
    /// Optional sandbox name. When set, the spawned session runs inside a Docker sandbox.
    pub sandbox: Option<String>,
    /// Path to the `zag` executable to launch as the background child.
    ///
    /// When `None`, [`resolve_zag_bin`] is used to discover one in this order:
    /// the `ZAG_BIN` environment variable, then a `zag` binary on `PATH`, then
    /// `std::env::current_exe()` if the current executable *is* `zag`. Library
    /// callers whose host process is not `zag` should set this (or `ZAG_BIN`)
    /// explicitly — otherwise spawn will fail with a clear error.
    pub zag_bin: Option<PathBuf>,
}

/// Directory for spawn log files.
fn spawn_logs_dir() -> PathBuf {
    // Per-user log directory override (set by zag serve in user-account mode)
    if let Ok(user_log_dir) = std::env::var("ZAG_USER_LOG_DIR") {
        return PathBuf::from(user_log_dir).join("spawn");
    }
    Config::global_base_dir().join("logs").join("spawn")
}

/// Directory for session FIFOs (interactive sessions).
fn fifos_dir() -> PathBuf {
    Config::global_base_dir().join("fifos")
}

/// Get the FIFO path for a given session ID.
pub fn fifo_path(session_id: &str) -> PathBuf {
    fifos_dir().join(session_id)
}

/// Walk `PATH` looking for an executable named `name`. Returns `None` when
/// none of the entries contain an executable file with that name.
///
/// On Unix we check the `x` bit via `fs::metadata`; on other platforms we
/// fall back to existence only, which is the same contract callers get from
/// `std::process::Command` when a binary lookup fails.
fn find_on_path(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if !candidate.is_file() {
            continue;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(&candidate)
                && meta.permissions().mode() & 0o111 != 0
            {
                return Some(candidate);
            }
        }
        #[cfg(not(unix))]
        {
            return Some(candidate);
        }
    }
    None
}

/// Resolve which `zag` binary to invoke as the background child.
///
/// Resolution order (first hit wins):
/// 1. `explicit` — typically `SpawnParams::zag_bin`.
/// 2. The `ZAG_BIN` environment variable (when non-empty).
/// 3. A `zag` executable on `PATH`.
/// 4. `std::env::current_exe()`, **only** when the current executable's
///    file stem is `zag`. This is the path the `zag` CLI itself takes.
///
/// Library callers embedding `zag-orch` from a non-`zag` host process must
/// set `zag_bin` or `ZAG_BIN`; otherwise this returns an error describing
/// what to configure. This replaces the old behavior, which silently used
/// `current_exe()` and produced a broken subprocess whose argv was the
/// caller's own binary.
pub fn resolve_zag_bin(explicit: Option<&std::path::Path>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        return Ok(p.to_path_buf());
    }
    if let Ok(env_val) = std::env::var("ZAG_BIN")
        && !env_val.is_empty()
    {
        return Ok(PathBuf::from(env_val));
    }
    if let Some(p) = find_on_path("zag") {
        return Ok(p);
    }
    if let Ok(current) = std::env::current_exe() {
        let is_zag = current
            .file_stem()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s == "zag");
        if is_zag {
            return Ok(current);
        }
    }
    anyhow::bail!(
        "No `zag` binary found. Set `SpawnParams::zag_bin`, set `ZAG_BIN` in the environment, \
         or install `zag` on PATH."
    )
}

/// Result of spawning a session.
#[derive(Debug, serde::Serialize)]
pub struct SpawnResult {
    pub session_id: String,
    pub pid: u32,
    pub log_path: String,
    pub interactive: bool,
}

/// Build common agent args shared between exec and relay modes.
fn build_agent_args(params: &SpawnParams) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();

    // Quiet mode for background
    args.push("--quiet".to_string());

    // Provider
    args.push("-p".to_string());
    args.push(params.provider.clone());

    // Model
    if let Some(ref model) = params.model {
        args.push("--model".to_string());
        args.push(model.clone());
    }

    // Root
    if let Some(ref root) = params.root {
        args.push("--root".to_string());
        args.push(root.clone());
    }

    // Auto approve
    if params.auto_approve {
        args.push("--auto-approve".to_string());
    }

    // System prompt
    if let Some(ref sp) = params.system_prompt {
        args.push("--system-prompt".to_string());
        args.push(sp.clone());
    }

    // Add dirs
    for dir in &params.add_dirs {
        args.push("--add-dir".to_string());
        args.push(dir.clone());
    }

    // Size (ollama)
    if let Some(ref size) = params.size {
        args.push("--size".to_string());
        args.push(size.clone());
    }

    args
}

/// Build isolation args (sandbox) that go after the subcommand.
fn build_isolation_args(params: &SpawnParams) -> Vec<String> {
    let mut args = Vec::new();
    if let Some(ref name) = params.sandbox {
        args.push("--sandbox".to_string());
        args.push(name.clone());
    }
    args
}

/// Build args for interactive relay mode.
fn build_relay_args(params: &SpawnParams, session_id: &str) -> Vec<String> {
    let mut args = build_agent_args(params);

    args.push("relay".to_string());
    args.push("--session".to_string());
    args.push(session_id.to_string());

    // Optional initial prompt
    if let Some(ref prompt) = params.prompt {
        args.push(prompt.clone());
    }

    args
}

/// Build args for non-interactive exec mode.
fn build_exec_args(params: &SpawnParams, session_id: &str) -> Vec<String> {
    let mut args = build_agent_args(params);

    // Exec subcommand with session ID
    args.push("exec".to_string());
    args.extend(build_isolation_args(params));
    args.push("--session".to_string());
    args.push(session_id.to_string());

    // Max turns
    if let Some(max_turns) = params.max_turns {
        args.push("--max-turns".to_string());
        args.push(max_turns.to_string());
    }

    // Timeout
    if let Some(ref timeout) = params.timeout {
        args.push("--timeout".to_string());
        args.push(timeout.clone());
    }

    // Session metadata
    if let Some(ref name) = params.metadata.name {
        args.push("--name".to_string());
        args.push(name.clone());
    }
    if let Some(ref desc) = params.metadata.description {
        args.push("--description".to_string());
        args.push(desc.clone());
    }
    for tag in &params.metadata.tags {
        args.push("--tag".to_string());
        args.push(tag.clone());
    }

    // The prompt (required for exec mode)
    if let Some(ref prompt) = params.prompt {
        args.push(prompt.clone());
    }

    args
}

/// Create a FIFO (named pipe) for the given session.
fn create_fifo(session_id: &str) -> Result<PathBuf> {
    #[cfg(not(unix))]
    {
        let _ = session_id;
        anyhow::bail!("Interactive sessions require a Unix-like OS (FIFOs not available)");
    }

    #[cfg(unix)]
    {
        let dir = fifos_dir();
        fs::create_dir_all(&dir)?;
        let path = dir.join(session_id);
        // Remove stale FIFO if it exists
        let _ = fs::remove_file(&path);
        nix::unistd::mkfifo(
            &path,
            nix::sys::stat::Mode::S_IRUSR | nix::sys::stat::Mode::S_IWUSR,
        )?;
        Ok(path)
    }
}

/// Build the `Command` that spawns a background session, without running it.
///
/// Returns the fully-configured `Command` (binary, argv, env vars, stdout/stderr
/// redirected to a new log file) plus the log file path. Useful for callers
/// that want to wrap the spawn in a supervisor (systemd-run, nohup, a custom
/// process group) or capture the exact argv before executing.
///
/// The caller is still responsible for setting `Stdio` on stdin and calling
/// `.spawn()`; [`spawn_session`] is the standard wrapper that does both plus
/// session-store / process-store registration.
///
/// **Note:** this does *not* create the FIFO for `params.interactive` — that
/// side effect is intentional and lives in `spawn_session`, because it should
/// only happen when the caller actually plans to execute the command.
pub fn build_spawn_command(
    params: &SpawnParams,
    session_id: &str,
) -> Result<(std::process::Command, PathBuf)> {
    let zag_bin = resolve_zag_bin(params.zag_bin.as_deref())?;

    let mut args = if params.interactive {
        build_relay_args(params, session_id)
    } else {
        build_exec_args(params, session_id)
    };

    let logs_dir = spawn_logs_dir();
    fs::create_dir_all(&logs_dir)?;
    let log_path = logs_dir.join(format!("{session_id}.log"));
    let stdout_file = File::create(&log_path)?;
    let stderr_file = stdout_file.try_clone()?;

    // If --inject-context is set, add --context for each dependency (exec mode only).
    if params.inject_context && !params.interactive {
        for dep in &params.depends_on {
            let prompt = args.pop().unwrap();
            args.push("--context".to_string());
            args.push(dep.clone());
            args.push(prompt);
        }
    }

    debug!("Spawning: {} {}", zag_bin.display(), args.join(" "));

    let mut cmd = if !params.depends_on.is_empty() && !params.interactive {
        // Dependencies (exec mode): wrap in a shell that waits first.
        let wait_args: Vec<String> = params
            .depends_on
            .iter()
            .map(|id| format!("\"{id}\""))
            .collect();
        let wait_cmd = format!(
            "{} wait {} && {} {}",
            zag_bin.display(),
            wait_args.join(" "),
            zag_bin.display(),
            args.iter()
                .map(|a| format!("\"{}\"", a.replace('"', "\\\"")))
                .collect::<Vec<_>>()
                .join(" ")
        );
        debug!("Spawn with deps: sh -c '{wait_cmd}'");
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-c").arg(&wait_cmd);
        cmd
    } else {
        let mut cmd = std::process::Command::new(&zag_bin);
        cmd.args(&args);
        cmd
    };

    cmd.stdout(stdout_file).stderr(stderr_file);
    for (key, val) in &params.env_vars {
        cmd.env(key, val);
    }

    Ok((cmd, log_path))
}

/// Spawn a background session, returning structured result.
pub fn spawn_session(params: &SpawnParams) -> Result<SpawnResult> {
    let session_id = uuid::Uuid::new_v4().to_string();
    let workspace = current_workspace(params.root.as_deref());

    log::info!(
        "Spawning session {}: provider={} model={} interactive={} prompt={}",
        session_id,
        params.provider,
        params.model.as_deref().unwrap_or("default"),
        params.interactive,
        params.prompt.as_deref().unwrap_or("(none)")
    );

    // Register in session store
    let mut session_store = SessionStore::load(params.root.as_deref()).unwrap_or_default();
    session_store.add(SessionEntry {
        session_id: session_id.clone(),
        provider: params.provider.clone(),
        model: params.model.clone().unwrap_or_default(),
        worktree_path: workspace.clone(),
        worktree_name: String::new(),
        created_at: chrono::Utc::now().to_rfc3339(),
        provider_session_id: None,
        sandbox_name: params.sandbox.clone(),
        is_worktree: false,
        discovered: false,
        discovery_source: None,
        log_path: None,
        log_completeness: "partial".to_string(),
        name: params.metadata.name.clone(),
        description: params.metadata.description.clone(),
        tags: params.metadata.tags.clone(),
        dependencies: params.depends_on.clone(),
        retried_from: params.retried_from.clone(),
        interactive: params.interactive,
    });
    if let Err(e) = session_store.save(params.root.as_deref()) {
        log::warn!("Failed to save session store: {e}");
    }

    // Create FIFO for interactive sessions
    if params.interactive {
        create_fifo(&session_id)?;
        debug!("Created FIFO at {}", fifo_path(&session_id).display());
    }

    let (mut cmd, log_path) = build_spawn_command(params, &session_id)?;
    cmd.stdin(std::process::Stdio::null());
    let child = cmd.spawn()?;

    let child_pid = child.id();

    // Register in process store
    let command = if params.interactive {
        "interactive"
    } else {
        "exec"
    };
    let mut proc_store = ProcessStore::load().unwrap_or_default();
    proc_store.add(ProcessEntry {
        id: uuid::Uuid::new_v4().to_string(),
        pid: child_pid,
        session_id: Some(session_id.clone()),
        provider: params.provider.clone(),
        model: params.model.clone().unwrap_or_default(),
        command: command.to_string(),
        prompt: params
            .prompt
            .as_ref()
            .map(|p| p.chars().take(100).collect()),
        started_at: chrono::Utc::now().to_rfc3339(),
        status: "running".to_string(),
        exit_code: None,
        exited_at: None,
        root: Some(workspace),
        parent_process_id: std::env::var("ZAG_PROCESS_ID").ok(),
        parent_session_id: std::env::var("ZAG_SESSION_ID").ok(),
    });
    if let Err(e) = proc_store.save() {
        log::warn!("Failed to save process store: {e}");
    }

    Ok(SpawnResult {
        session_id,
        pid: child_pid,
        log_path: log_path.to_string_lossy().to_string(),
        interactive: params.interactive,
    })
}

/// Run the spawn command (print output wrapper).
pub fn run_spawn(params: SpawnParams) -> Result<()> {
    let json = params.json;
    let result = spawn_session(&params)?;

    if json {
        println!("{}", serde_json::to_string(&result)?);
    } else {
        println!("{}", result.session_id);
    }

    Ok(())
}

#[cfg(test)]
#[path = "spawn_tests.rs"]
mod tests;
