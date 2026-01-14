use env_logger::Builder;
use indicatif::{ProgressBar, ProgressStyle};
use log::LevelFilter;
use std::io::Write;

/// Initialize logging with the specified debug level
pub fn init(debug: bool) {
    let level = if debug {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    Builder::new()
        .filter_level(level)
        .format(|buf, record| {
            match record.level() {
                log::Level::Debug => writeln!(buf, "[DEBUG] {}", record.args()),
                log::Level::Info => writeln!(buf, "\x1b[33m>\x1b[0m {}", record.args()),
                _ => writeln!(buf, "{}", record.args()),
            }
        })
        .init();
}

/// Create a spinner with a message
pub fn spinner(msg: impl Into<String>) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    pb.set_message(msg.into());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}

/// Finish a spinner quietly (just clear it)
pub fn finish_spinner_quiet(pb: &ProgressBar) {
    pb.finish_and_clear();
}
