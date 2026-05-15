//! Headless interactive spawn: attach a provider's TUI to a private
//! pseudo-terminal so the operator never sees it.
//!
//! Used by every provider's interactive path when the user passes
//! `--headless`. Required pair: `-a` (so the hidden TUI doesn't block on
//! permission prompts) and `--exit` (so the run has a defined termination
//! and result-capture signal via `zag ps kill self`).
//!
//! ## How termination interacts with `zag ps kill self`
//!
//! The `on_spawn` hook fires with the PID of the agent CLI child
//! process — exactly the same PID the non-headless path registers. So
//! `zag ps kill self <result>` continues to SIGTERM the agent directly
//! rather than the PTY. When the agent exits, the slave side is dropped,
//! the master gets EOF, and the drain thread terminates. The PTY itself
//! is reaped by the OS along with the child.
//!
//! Note: portable_pty's `Child` is blocking, so we wait on it inside
//! `tokio::task::spawn_blocking` to avoid stalling the tokio runtime.

use crate::agent::OnSpawnHook;
use anyhow::{Context, Result};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::ffi::OsString;
use tokio::process::Command as TokioCommand;

/// Spawn `cmd` attached to a private PTY, drain and discard its output,
/// and wait for it to exit. Returns `ProcessError` on non-zero exit so the
/// error path matches the inherit-stdio call site.
pub async fn spawn_headless(
    cmd: &mut TokioCommand,
    agent_display_name: &str,
    on_spawn: Option<&OnSpawnHook>,
) -> Result<()> {
    let display = agent_display_name.to_string();

    // Convert tokio::process::Command into portable_pty::CommandBuilder.
    // tokio::Command derefs to std::Command which exposes the parts.
    let std_cmd = cmd.as_std();
    let program: OsString = std_cmd.get_program().to_owned();
    let args: Vec<OsString> = std_cmd.get_args().map(|s| s.to_owned()).collect();
    let cwd = std_cmd.get_current_dir().map(|p| p.to_owned());
    let envs: Vec<(OsString, OsString)> = std_cmd
        .get_envs()
        .filter_map(|(k, v)| v.map(|val| (k.to_owned(), val.to_owned())))
        .collect();

    let mut builder = CommandBuilder::new(&program);
    builder.args(&args);
    if let Some(dir) = cwd {
        builder.cwd(dir);
    }
    for (k, v) in envs {
        builder.env(k, v);
    }

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .with_context(|| format!("Failed to allocate PTY for headless {display}"))?;

    let mut child = pair.slave.spawn_command(builder).with_context(|| {
        format!(
            "Failed to execute '{}' CLI under headless PTY. Is it installed and in PATH?",
            display.to_lowercase()
        )
    })?;

    if let (Some(cb), Some(pid)) = (on_spawn, child.process_id()) {
        cb(pid);
    }

    // Drop the slave so the master observes EOF when the child exits.
    drop(pair.slave);

    // Drain and discard everything the TUI writes to the PTY master.
    // Without this the child can block once the kernel pty buffer fills.
    let mut reader = pair
        .master
        .try_clone_reader()
        .context("Failed to clone PTY master reader")?;
    std::thread::spawn(move || {
        use std::io::Read;
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(_) => continue,
                Err(_) => break,
            }
        }
    });

    // portable_pty's Child is blocking, so wait off the tokio runtime.
    let display_for_wait = display.clone();
    let status = tokio::task::spawn_blocking(move || child.wait())
        .await
        .with_context(|| format!("Failed joining headless PTY waiter for {display_for_wait}"))?
        .with_context(|| format!("Failed waiting on '{}' CLI", display.to_lowercase()))?;

    if !status.success() {
        return Err(crate::process::ProcessError {
            exit_code: status.exit_code().try_into().ok(),
            stderr: String::new(),
            agent_name: display,
        }
        .into());
    }
    Ok(())
}

#[cfg(test)]
#[path = "headless_pty_tests.rs"]
mod tests;
