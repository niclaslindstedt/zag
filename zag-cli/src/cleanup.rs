//! Worktree and sandbox cleanup prompts after agent sessions.

#[cfg(test)]
#[path = "cleanup_tests.rs"]
mod tests;

use crate::sandbox;
use crate::session;
use crate::worktree;
use anyhow::Result;
use log::debug;

/// Prompt the user whether to keep or remove a sandbox after an interactive session.
pub fn prompt_sandbox_cleanup(
    session_id: &str,
    sandbox_name: &str,
    root: Option<&str>,
) -> Result<()> {
    use std::io::{self, BufRead, Write};

    debug!("Prompting sandbox cleanup: session={session_id}, sandbox={sandbox_name}");
    println!("\n\x1b[33m>\x1b[0m Sandbox: {sandbox_name}");
    print!("\x1b[33m>\x1b[0m Keep sandbox? [Y/n] ");
    io::stdout().flush()?;

    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    let answer = line.trim().to_lowercase();

    if answer == "n" || answer == "no" {
        match sandbox::remove_sandbox(sandbox_name) {
            Ok(()) => {
                println!("\x1b[32m✓\x1b[0m Sandbox removed");
            }
            Err(e) => {
                log::warn!("Failed to remove sandbox: {e}");
                println!("\x1b[31m✗\x1b[0m Failed to remove sandbox: {e}");
            }
        }
        // Remove session mapping
        let mut store = session::SessionStore::load(root).unwrap_or_default();
        store.remove(session_id);
        let _ = store.save(root);
    } else {
        let store = session::SessionStore::load(root).unwrap_or_default();
        let provider_session_id = store
            .find_by_session_id(session_id)
            .and_then(|entry| entry.provider_session_id.as_deref());
        print_resume_hint(session_id, provider_session_id, "Sandbox");
    }

    Ok(())
}

/// Prompt the user whether to keep or delete a worktree after an interactive session.
pub fn prompt_worktree_cleanup(
    session_id: &str,
    worktree_path: &str,
    root: Option<&str>,
) -> Result<()> {
    use std::io::{self, BufRead, Write};

    debug!("Prompting worktree cleanup: session={session_id}, path={worktree_path}");
    println!("\n\x1b[33m>\x1b[0m Worktree at {worktree_path}");
    print!("\x1b[33m>\x1b[0m Keep workspace? [Y/n] ");
    io::stdout().flush()?;

    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    let answer = line.trim().to_lowercase();

    if answer == "n" || answer == "no" {
        let wt_path = std::path::Path::new(worktree_path);
        if wt_path.exists() {
            match worktree::remove_worktree(wt_path) {
                Ok(()) => {
                    println!("\x1b[32m✓\x1b[0m Worktree removed");
                }
                Err(e) => {
                    log::warn!("Failed to remove worktree: {e}");
                    println!("\x1b[31m✗\x1b[0m Failed to remove worktree: {e}");
                }
            }
        }
        // Remove session mapping
        let mut store = session::SessionStore::load(root).unwrap_or_default();
        store.remove(session_id);
        let _ = store.save(root);
    } else {
        let store = session::SessionStore::load(root).unwrap_or_default();
        let provider_session_id = store
            .find_by_session_id(session_id)
            .and_then(|entry| entry.provider_session_id.as_deref());
        print_resume_hint(session_id, provider_session_id, "Workspace");
    }

    Ok(())
}

pub fn print_resume_hint(wrapper_session_id: &str, provider_session_id: Option<&str>, label: &str) {
    println!("\x1b[32m✓\x1b[0m {label} kept. Resume with: zag run --resume {wrapper_session_id}");
    if let Some(provider_session_id) = provider_session_id
        && provider_session_id != wrapper_session_id
    {
        println!("\x1b[32m✓\x1b[0m Native provider ID: {provider_session_id}");
    }
}

/// Prints a session resume hint after exiting an interactive session.
pub fn print_session_resume_hint(wrapper_session_id: &str, provider_session_id: Option<&str>) {
    println!();
    println!("Resume this session: \x1b[36mzag run --resume {wrapper_session_id}\x1b[0m");
    if let Some(provider_session_id) = provider_session_id
        && provider_session_id != wrapper_session_id
    {
        println!("   (native provider ID: {provider_session_id})");
    }
}
