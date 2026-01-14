use std::sync::atomic::{AtomicBool, Ordering};
use tokio::signal;

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

/// Initialize the interrupt handler for workflows.
/// CTRL+C (SIGINT) sets the interrupted flag, allowing graceful shutdown.
/// The workflow will mark the current phase as failed and be resumable.
pub fn init() {
    tokio::spawn(async {
        signal::ctrl_c().await.ok();
        INTERRUPTED.store(true, Ordering::SeqCst);
        println!("\nInterrupted - workflow will be resumable");
    });
}

/// Check if the process was interrupted by SIGINT.
pub fn was_interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

/// Set the interrupted flag manually.
/// Used when we detect SIGINT via process exit status before the signal handler runs.
pub fn set_interrupted() {
    INTERRUPTED.store(true, Ordering::SeqCst);
}
