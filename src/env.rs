//! Env command: export session environment variables for nested agent invocations.

use anyhow::Result;
use zag::process_store::ProcessStore;
use zag::session::SessionStore;

/// Run the env command.
pub fn run_env(session_id: Option<&str>, shell: bool, root: Option<&str>) -> Result<()> {
    let session_store = SessionStore::load(root).unwrap_or_default();
    let proc_store = ProcessStore::load().unwrap_or_default();

    // Resolve session
    let session = if let Some(id) = session_id {
        session_store
            .find_by_any_id(id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", id))?
    } else {
        session_store
            .latest()
            .ok_or_else(|| anyhow::anyhow!("No sessions found"))?
    };

    // Find matching process entry
    let proc_entry = proc_store
        .processes
        .iter()
        .filter(|e| e.session_id.as_deref() == Some(&session.session_id))
        .max_by(|a, b| a.started_at.cmp(&b.started_at));

    let mut vars: Vec<(&str, String)> = Vec::new();

    vars.push(("ZAG_SESSION_ID", session.session_id.clone()));

    if let Some(ref name) = session.name {
        vars.push(("ZAG_SESSION_NAME", name.clone()));
    }

    if let Some(pe) = proc_entry {
        vars.push(("ZAG_PROCESS_ID", pe.id.clone()));
    }

    vars.push(("ZAG_PROVIDER", session.provider.clone()));

    if !session.model.is_empty() {
        vars.push(("ZAG_MODEL", session.model.clone()));
    }

    if !session.worktree_path.is_empty() {
        vars.push(("ZAG_ROOT", session.worktree_path.clone()));
    }

    for (key, value) in &vars {
        if shell {
            println!("export {}={};", key, shell_escape(value));
        } else {
            println!("{}={}", key, value);
        }
    }

    Ok(())
}

/// Escape a value for shell export.
fn shell_escape(s: &str) -> String {
    if s.contains('\'') {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        format!("'{}'", s)
    }
}

#[cfg(test)]
#[path = "env_tests.rs"]
mod tests;
