use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

/// Build a `tokio::process::Command` that prints a short message and exits 0.
fn trivial_success_command() -> tokio::process::Command {
    #[cfg(windows)]
    {
        let mut cmd = tokio::process::Command::new("cmd.exe");
        cmd.args(["/C", "echo zag-headless-smoke"]);
        cmd
    }
    #[cfg(not(windows))]
    {
        let mut cmd = tokio::process::Command::new("/bin/echo");
        cmd.arg("zag-headless-smoke");
        cmd
    }
}

/// Build a `tokio::process::Command` that exits with code 7.
fn trivial_failure_command() -> tokio::process::Command {
    #[cfg(windows)]
    {
        let mut cmd = tokio::process::Command::new("cmd.exe");
        cmd.args(["/C", "exit 7"]);
        cmd
    }
    #[cfg(not(windows))]
    {
        let mut cmd = tokio::process::Command::new("/bin/sh");
        cmd.args(["-c", "exit 7"]);
        cmd
    }
}

/// Smoke test the PTY pipeline end-to-end with a trivial command.
///
/// Verifies that:
/// - the child runs to a successful exit through the PTY,
/// - the `on_spawn` hook fires with a non-zero pid (so
///   `zag ps kill self` would have something to target),
/// - the drain thread and blocking waiter shut down cleanly when the
///   child exits and the slave handle is dropped.
#[tokio::test]
async fn spawn_headless_runs_trivial_command_to_success() {
    let mut cmd = trivial_success_command();

    let captured_pid = Arc::new(AtomicU32::new(0));
    let captured_pid_for_hook = captured_pid.clone();
    let hook: crate::agent::OnSpawnHook = Arc::new(move |pid: u32| {
        captured_pid_for_hook.store(pid, Ordering::SeqCst);
    });

    spawn_headless(&mut cmd, "Echo", Some(&hook))
        .await
        .expect("headless spawn should succeed for trivial echo command");

    let pid = captured_pid.load(Ordering::SeqCst);
    assert!(
        pid > 0,
        "on_spawn hook should have fired with a real pid (got {pid})"
    );
}

/// Non-zero exit must surface as `ProcessError`, matching the inherit-stdio
/// path's error contract — so `--exit`'s SIGTERM-as-success logic and other
/// callers see a uniform error shape regardless of headless mode.
#[tokio::test]
async fn spawn_headless_reports_process_error_on_nonzero_exit() {
    let mut cmd = trivial_failure_command();

    let err = spawn_headless(&mut cmd, "Sh", None)
        .await
        .expect_err("non-zero exit should error");

    let process_err = err
        .downcast_ref::<crate::process::ProcessError>()
        .expect("error should be a ProcessError");
    assert_eq!(process_err.exit_code, Some(7));
    assert_eq!(process_err.agent_name, "Sh");
}
