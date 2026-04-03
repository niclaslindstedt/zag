use crate::config::Config;
use env_logger::Builder;
use indicatif::{ProgressBar, ProgressStyle};
use log::LevelFilter;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag to track if quiet mode is enabled
static QUIET_MODE: AtomicBool = AtomicBool::new(false);

/// Global log file handle
static LOG_FILE: Mutex<Option<File>> = Mutex::new(None);

/// Initialize logging with the specified debug and quiet levels.
/// Also creates a session log file in the global logs directory.
pub fn init(debug: bool, quiet: bool) {
    // Store quiet mode state globally
    QUIET_MODE.store(quiet, Ordering::Relaxed);

    // Initialize file-based logging
    init_log_file();

    // In quiet mode, disable all terminal logging
    let level = if quiet {
        LevelFilter::Off
    } else if debug {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    Builder::new()
        .filter_level(level)
        .format(|buf, record| {
            // Always write to log file regardless of terminal log level
            write_to_log_file(&format!("[{}] {}", record.level(), record.args()));

            match record.level() {
                log::Level::Debug => writeln!(buf, "\x1b[90m*\x1b[0m {}", record.args()), // Dim gray asterisk
                log::Level::Info => writeln!(buf, "\x1b[33m>\x1b[0m {}", record.args()), // Orange arrow
                log::Level::Warn => writeln!(buf, "\x1b[93m!\x1b[0m {}", record.args()), // Bright yellow exclamation
                log::Level::Error => writeln!(buf, "\x1b[91m✗\x1b[0m {}", record.args()), // Bright red X
                log::Level::Trace => writeln!(buf, "\x1b[90m·\x1b[0m {}", record.args()), // Dim gray dot
            }
        })
        .init();
}

/// Initialize the log file for this session.
fn init_log_file() {
    let logs_dir = Config::global_logs_dir();
    if fs::create_dir_all(&logs_dir).is_err() {
        return;
    }

    let timestamp = chrono::Local::now().format("%Y-%m-%dT%H-%M-%S");
    let log_path = logs_dir.join(format!("zag-{}.log", timestamp));

    if let Ok(file) = OpenOptions::new().create(true).append(true).open(&log_path)
        && let Ok(mut guard) = LOG_FILE.lock()
    {
        *guard = Some(file);
    }
}

/// Write a message to the log file.
fn write_to_log_file(msg: &str) {
    if let Ok(mut guard) = LOG_FILE.lock()
        && let Some(ref mut file) = *guard
    {
        let timestamp = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f");
        let _ = writeln!(file, "{} {}", timestamp, msg);
    }
}

/// Check if quiet mode is enabled
pub fn is_quiet() -> bool {
    QUIET_MODE.load(Ordering::Relaxed)
}

/// Create a spinner with a message (returns hidden spinner if in quiet mode)
pub fn spinner(msg: impl Into<String>) -> ProgressBar {
    let pb = ProgressBar::new_spinner();

    // If quiet mode is enabled, return a hidden progress bar
    if is_quiet() {
        pb.set_draw_target(indicatif::ProgressDrawTarget::hidden());
        return pb;
    }

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
