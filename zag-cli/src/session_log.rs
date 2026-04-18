pub use zag_agent::session_log::*;

use crate::config::Config;
use anyhow::Result;
use std::path::PathBuf;

pub fn logs_dir(root: Option<&str>) -> PathBuf {
    // Per-user log directory override (set by zag serve in user-account mode)
    if let Ok(user_log_dir) = std::env::var("ZAG_USER_LOG_DIR") {
        return PathBuf::from(user_log_dir);
    }
    Config::agent_dir(root).join("logs")
}

pub fn run_default_backfill(root: Option<&str>) -> Result<usize> {
    let claude = crate::claude::logs::ClaudeHistoricalLogAdapter;
    let codex = crate::codex::CodexHistoricalLogAdapter;
    let gemini = crate::gemini::GeminiHistoricalLogAdapter;
    let copilot = crate::copilot::CopilotHistoricalLogAdapter;
    let ollama = crate::ollama::OllamaHistoricalLogAdapter;
    let providers: [&dyn HistoricalLogAdapter; 5] = [&claude, &codex, &gemini, &copilot, &ollama];
    run_backfill(&logs_dir(root), root, &providers)
}

#[cfg(test)]
#[path = "session_log_tests.rs"]
mod tests;
