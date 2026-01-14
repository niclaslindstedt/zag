use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::signal;

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

/// Initialize the interrupt handler for workflows.
/// CTRL+C (SIGINT) sets the interrupted flag.
/// The actual prompt happens when the interrupt is checked.
pub fn init() {
    tokio::spawn(async {
        signal::ctrl_c().await.ok();
        INTERRUPTED.store(true, Ordering::SeqCst);
    });
}

/// Prompt user to choose what to do after CTRL+C.
/// Returns true if user wants to continue to next phase (with checkpoint).
pub fn prompt_interrupt_action() -> bool {
    println!("\n\n=== Interrupted (CTRL+C) ===");
    print!("Save progress and continue to next phase? [y/N]: ");
    io::stdout().flush().ok();

    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_) => {
            match input.trim().to_lowercase().as_str() {
                "y" | "yes" => {
                    println!("Checkpointing and continuing to next phase...");
                    true
                }
                _ => {
                    println!("Exiting without checkpoint...");
                    false
                }
            }
        }
        Err(_) => false,
    }
}

/// Check if the process was interrupted by SIGINT.
pub fn was_interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

/// Clear the interrupt state.
pub fn clear_interrupt() {
    INTERRUPTED.store(false, Ordering::SeqCst);
}

/// Set the interrupted flag manually.
/// Used when we detect SIGINT via process exit status before the signal handler runs.
pub fn set_interrupted() {
    INTERRUPTED.store(true, Ordering::SeqCst);
}
