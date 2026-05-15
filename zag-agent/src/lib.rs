pub mod agent;
pub mod attachment;
pub mod auto_selector;
pub mod builder;
pub mod capability;
pub mod config;
pub mod exit_mode;
pub mod factory;
pub mod file_util;
pub mod headless_pty;
pub mod json_validation;
pub mod listen;
pub mod manpages;
pub mod mcp;
pub mod output;
pub mod plan;
pub mod preflight;
pub mod process;
pub mod process_registration;
pub mod process_store;
pub mod progress;
pub mod prompts;
pub mod providers;
pub mod review;
pub mod sandbox;
pub mod search;
pub mod session;
pub mod session_log;
pub mod skills;
pub mod streaming;
pub mod usage_limits;
pub mod worktree;

/// Truncate a string to at most `max_bytes` bytes, rounding down to a valid
/// UTF-8 char boundary. Equivalent to `str::floor_char_boundary` (Rust 1.91+).
pub(crate) fn truncate_str(s: &str, max_bytes: usize) -> &str {
    if max_bytes >= s.len() {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

#[cfg(test)]
#[path = "mock_integration_tests.rs"]
mod mock_integration_tests;
