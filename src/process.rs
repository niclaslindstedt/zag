use anyhow::Result;
use tokio::process::Child;
use tokio::signal;

use crate::interrupt;
use crate::pid;

/// Check if the process was terminated by SIGINT (Ctrl+C).
#[cfg(unix)]
fn was_terminated_by_sigint(status: &std::process::ExitStatus) -> bool {
    use std::os::unix::process::ExitStatusExt;
    // SIGINT = 2 on Unix
    status.signal() == Some(2)
}

#[cfg(not(unix))]
fn was_terminated_by_sigint(_status: &std::process::ExitStatus) -> bool {
    false
}

/// Wait for a child process to complete.
/// Writes the child's PID so `agent exit` can terminate it directly.
/// Uses select! to race between child completion and CTRL+C signal.
///
/// If `require_explicit_completion` is true (for interactive phases), the agent
/// must call `agent exit` to signal successful completion. Otherwise, exiting
/// without `agent exit` is treated as a failure.
pub async fn wait_with_pid_tracking(
    mut child: Child,
    require_explicit_completion: bool,
) -> Result<()> {
    // Write child's PID so `agent exit` targets the agent CLI, not the parent
    if let Some(child_pid) = child.id() {
        let _ = pid::write_pid_for(child_pid);
    }

    // Race between child completing and CTRL+C signal
    let (status, ctrl_c_received) = tokio::select! {
        biased;  // Prefer ctrl_c branch when both are ready

        // CTRL+C received - wait for child to exit then return interrupted
        _ = signal::ctrl_c() => {
            interrupt::set_interrupted();
            // Wait for child to actually exit (it received SIGINT too)
            let status = child.wait().await;
            (status, true)
        }
        // Child process completed
        result = child.wait() => {
            (result, false)
        }
    };

    let status = status?;

    // Clean up PID file
    let _ = pid::remove_pid();

    // If ctrl_c branch won the race, we're definitely interrupted
    if ctrl_c_received {
        anyhow::bail!("Session was interrupted by Ctrl+C");
    }

    // Check if exited via `agent exit` - this is intentional termination
    // and should succeed to allow workflow continuation
    let was_killed = pid::was_killed();
    if was_killed {
        return Ok(());
    }

    // Check if the process was terminated by SIGINT signal (Ctrl+C).
    if was_terminated_by_sigint(&status) {
        interrupt::set_interrupted();
        anyhow::bail!("Session was interrupted by Ctrl+C");
    }

    // Give a small window for the interrupt handler to process SIGINT.
    // This handles the case where the child exits quickly after receiving
    // SIGINT (e.g., Claude Code handles it gracefully and exits with 0).
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Check if interrupted flag was set (by init() handler or signal)
    if interrupt::was_interrupted() {
        anyhow::bail!("Session was interrupted");
    }

    if !status.success() {
        anyhow::bail!("process exited with status: {}", status);
    }

    // For interactive phases, the agent MUST call `agent exit` to signal
    // successful completion. If it just exits (even with status 0), treat
    // it as a failure - the agent didn't follow the completion instructions.
    if require_explicit_completion && !was_killed {
        anyhow::bail!("Agent exited without signaling completion (did not run `agent exit`)");
    }

    Ok(())
}
