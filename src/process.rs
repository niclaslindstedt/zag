use anyhow::Result;
use tokio::process::Child;

use crate::interrupt;
use crate::pid;

/// Wait for a child process to complete.
/// Writes the child's PID so `agent kill` can terminate it directly.
pub async fn wait_with_pid_tracking(mut child: Child) -> Result<()> {
    // Write child's PID so `agent kill` targets the agent CLI, not the parent
    if let Some(child_pid) = child.id() {
        let _ = pid::write_pid_for(child_pid);
    }

    let status = child.wait().await?;

    // Clean up PID file
    let _ = pid::remove_pid();

    // Yield to let interrupt handler task run if Ctrl+C was pressed
    tokio::task::yield_now().await;

    // Check if interrupted via Ctrl+C
    if interrupt::was_interrupted() {
        anyhow::bail!("Session was interrupted");
    }

    // Check if killed via `agent kill`
    if pid::was_killed() {
        // Session was intentionally killed - this is success for workflow continuation
        return Ok(());
    }

    if !status.success() {
        anyhow::bail!("process exited with status: {}", status);
    }

    Ok(())
}
