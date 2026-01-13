use anyhow::Result;
use tokio::process::Child;
use tokio::signal;

pub async fn wait_with_signal_handling(mut child: Child) -> Result<()> {
    tokio::select! {
        status = child.wait() => {
            let status = status?;
            if !status.success() {
                anyhow::bail!("process exited with status: {}", status);
            }
            Ok(())
        }
        _ = signal::ctrl_c() => {
            child.kill().await?;
            Ok(())
        }
    }
}
