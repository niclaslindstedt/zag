use tokio::signal;

/// Initialize the interrupt handler.
/// CTRL+C (SIGINT) will exit the program immediately.
/// This is distinct from `agent kill` which sends SIGTERM.
pub fn init() {
    tokio::spawn(async {
        signal::ctrl_c().await.ok();
        println!("\nInterrupted");
        std::process::exit(130); // 128 + SIGINT(2)
    });
}
