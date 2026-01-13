use anyhow::Result;
use tokio::process::Child;
use tokio::signal::unix::{signal, SignalKind};

/// Wait for a child process, handling SIGTERM (from `agent kill`).
/// SIGINT (CTRL+C) is handled globally by the interrupt handler.
pub async fn wait_with_signal_handling(mut child: Child) -> Result<()> {
    let mut sigterm = signal(SignalKind::terminate())?;

    tokio::select! {
        status = child.wait() => {
            let status = status?;
            if !status.success() {
                anyhow::bail!("process exited with status: {}", status);
            }
            Ok(())
        }
        _ = sigterm.recv() => {
            // SIGTERM from `agent kill` - gracefully terminate child and continue workflow
            child.kill().await?;
            Ok(())
        }
    }
}
