//! Helper for registering an agent process in zag's `ProcessStore` and
//! producing the env vars (`ZAG_PROCESS_ID`, `ZAG_SESSION_ID`, etc.) that
//! `zag ps kill self` / `zig self terminate` use to resolve the running
//! agent from inside the agent's own subshell.
//!
//! Two callers need this exact sequence:
//!
//! - `zag-cli/src/commands/agent_action.rs` — the `zag agent` CLI path.
//! - `zag-agent::AgentBuilder` — used by library consumers (e.g. `zig run`
//!   interactive steps in the `zig` workflow tool) that drive an agent
//!   programmatically without going through the `zag` CLI.
//!
//! Before this module existed, only `agent_action.rs` did the registration
//! (and via `unsafe std::env::set_var`, polluting the parent process's env).
//! Library callers got no registration and no env vars, so `zig self
//! terminate` failed with `Cannot resolve "self": ZAG_PROCESS_ID is not set`
//! from inside any zig-run interactive step.
//!
//! This module is now the single source of truth. Callers either:
//!
//! - Call [`register`] directly and wire the returned [`ProcessRegistration`]
//!   to their `Agent` / `AgentBuilder` via `set_env_vars` + `set_on_spawn_hook`
//!   (or [`AgentBuilder::env`] / [`AgentBuilder::on_spawn`] / the convenience
//!   [`AgentBuilder::register_process`]), then call
//!   [`ProcessRegistration::update_status`] when the agent exits.
//! - Or pass a [`RegisterOptionsOwned`] to [`AgentBuilder::register_process`]
//!   and let the builder handle registration + finalisation automatically.

use crate::agent::OnSpawnHook;
use crate::process_store::{ProcessEntry, ProcessStore};
use std::sync::Arc;

/// Borrowed options for [`register`]. Field semantics mirror
/// [`ProcessEntry`] one-for-one.
pub struct RegisterOptions<'a> {
    /// Provider name (e.g. `"claude"`, `"codex"`).
    pub provider: &'a str,
    /// Effective model string (e.g. `"sonnet"`).
    pub model: &'a str,
    /// Subcommand label stored in the entry: `"run"`, `"exec"`, `"plan"`, etc.
    /// Used by `zag ps` for display.
    pub command: &'a str,
    /// First ~100 chars of the prompt, if any.
    pub prompt_preview: Option<&'a str>,
    /// Session id this process is associated with (zag's internal session_id,
    /// not the provider-native one). Becomes `ZAG_SESSION_ID` for the child.
    pub session_id: Option<&'a str>,
    /// Optional human-friendly session name. Becomes `ZAG_SESSION_NAME`.
    pub session_name: Option<&'a str>,
    /// Project root passed through to the entry and `ZAG_ROOT`.
    pub root: Option<&'a str>,
}

/// Owned variant of [`RegisterOptions`] for storage on a builder before the
/// terminal method runs.
#[derive(Debug, Clone, Default)]
pub struct RegisterOptionsOwned {
    pub provider: String,
    pub model: String,
    pub command: String,
    pub prompt_preview: Option<String>,
    pub session_id: Option<String>,
    pub session_name: Option<String>,
    pub root: Option<String>,
}

impl RegisterOptionsOwned {
    pub(crate) fn as_borrowed(&self) -> RegisterOptions<'_> {
        RegisterOptions {
            provider: &self.provider,
            model: &self.model,
            command: &self.command,
            prompt_preview: self.prompt_preview.as_deref(),
            session_id: self.session_id.as_deref(),
            session_name: self.session_name.as_deref(),
            root: self.root.as_deref(),
        }
    }
}

/// A live process registration. Holds the generated `proc_id` and the env
/// vars to inject into the agent subprocess. Callers wire `env_vars()` and
/// `on_spawn_hook()` to the agent and call `update_status()` once the agent
/// exits.
pub struct ProcessRegistration {
    proc_id: String,
    env_vars: Vec<(String, String)>,
}

impl ProcessRegistration {
    /// The UUID assigned to this registration. Matches the `id` field of the
    /// `ProcessEntry` in zag's process store and the `ZAG_PROCESS_ID` env
    /// var injected into the child.
    pub fn proc_id(&self) -> &str {
        &self.proc_id
    }

    /// The env vars to inject into the agent subprocess so it can resolve
    /// `"self"` for `zag ps kill self` / `zig self terminate`. Use with
    /// [`crate::agent::Agent::set_env_vars`] (CLI path) or repeated
    /// [`crate::builder::AgentBuilder::env`] calls (library path).
    pub fn env_vars(&self) -> &[(String, String)] {
        &self.env_vars
    }

    /// `on_spawn` hook that retargets the registry entry's `pid` from zag's
    /// own pid (registered up-front so `zag ps` is populated immediately) to
    /// the actual agent subprocess pid. Wire via
    /// [`crate::agent::Agent::set_on_spawn_hook`] or
    /// [`crate::builder::AgentBuilder::on_spawn`].
    ///
    /// Without this, `zag ps kill self` would SIGTERM the parent zag/zig
    /// orchestrator instead of the agent child, taking the workflow down
    /// with it.
    pub fn on_spawn_hook(&self) -> OnSpawnHook {
        let proc_id = self.proc_id.clone();
        Arc::new(move |pid: u32| {
            if let Ok(mut pstore) = ProcessStore::load() {
                pstore.update_pid(&proc_id, pid);
                let _ = pstore.save();
            }
        })
    }

    /// Mark the registration's entry as finished. Pass `"exited"` for clean
    /// completion or `"killed"` when the agent crashed / was signalled.
    pub fn update_status(&self, status: &str, exit_code: Option<i32>) {
        if let Ok(mut pstore) = ProcessStore::load() {
            pstore.update_status(&self.proc_id, status, exit_code);
            let _ = pstore.save();
        }
    }
}

/// Build the env-var list for a registration. Pure: no I/O, no env reads.
/// Exposed so unit tests can assert the var set without touching the real
/// `~/.zag/processes.json` or mutating process-global env state.
pub(crate) fn build_env_vars(proc_id: &str, opts: &RegisterOptions<'_>) -> Vec<(String, String)> {
    let mut env_vars = Vec::with_capacity(6);
    if let Some(sid) = opts.session_id {
        env_vars.push(("ZAG_SESSION_ID".to_string(), sid.to_string()));
    }
    env_vars.push(("ZAG_PROCESS_ID".to_string(), proc_id.to_string()));
    env_vars.push(("ZAG_PROVIDER".to_string(), opts.provider.to_string()));
    env_vars.push(("ZAG_MODEL".to_string(), opts.model.to_string()));
    if let Some(r) = opts.root {
        env_vars.push(("ZAG_ROOT".to_string(), r.to_string()));
    }
    if let Some(name) = opts.session_name {
        env_vars.push(("ZAG_SESSION_NAME".to_string(), name.to_string()));
    }
    env_vars
}

/// Build a `ProcessEntry` for a fresh registration. Pure: takes parent
/// linkage explicitly so tests don't need to mutate `ZAG_PROCESS_ID` /
/// `ZAG_SESSION_ID` to exercise it.
pub(crate) fn build_entry(
    proc_id: &str,
    opts: &RegisterOptions<'_>,
    parent_process_id: Option<String>,
    parent_session_id: Option<String>,
) -> ProcessEntry {
    ProcessEntry {
        id: proc_id.to_string(),
        pid: std::process::id(),
        session_id: opts.session_id.map(String::from),
        provider: opts.provider.to_string(),
        model: opts.model.to_string(),
        command: opts.command.to_string(),
        prompt: opts.prompt_preview.map(String::from),
        started_at: chrono::Utc::now().to_rfc3339(),
        status: "running".to_string(),
        exit_code: None,
        exited_at: None,
        root: opts.root.map(String::from),
        parent_process_id,
        parent_session_id,
    }
}

/// Register a new process entry in zag's `ProcessStore` and return a
/// [`ProcessRegistration`] holding the proc_id and the env vars to inject
/// into the agent subprocess.
///
/// Reads `ZAG_PROCESS_ID` / `ZAG_SESSION_ID` from the current process env to
/// record parent-process linkage for nested invocations.
///
/// The entry is registered with `pid = std::process::id()` (the caller's
/// pid). Wire [`ProcessRegistration::on_spawn_hook`] to the agent so the pid
/// is retargeted to the agent subprocess once it spawns.
pub fn register(opts: RegisterOptions<'_>) -> ProcessRegistration {
    let proc_id = uuid::Uuid::new_v4().to_string();
    let parent_process_id = std::env::var("ZAG_PROCESS_ID").ok();
    let parent_session_id = std::env::var("ZAG_SESSION_ID").ok();

    let entry = build_entry(&proc_id, &opts, parent_process_id, parent_session_id);
    let env_vars = build_env_vars(&proc_id, &opts);

    if let Ok(mut pstore) = ProcessStore::load() {
        pstore.add(entry);
        let _ = pstore.save();
    }

    ProcessRegistration { proc_id, env_vars }
}

#[cfg(test)]
#[path = "process_registration_tests.rs"]
mod tests;
