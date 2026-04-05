//! Env command: export session environment variables for nested agent invocations.

use anyhow::Result;
use zag_agent::process_store::ProcessStore;
use zag_agent::session::SessionStore;

/// Get environment variables for a session as key-value pairs.
pub fn get_env_vars(session_id: Option<&str>, root: Option<&str>) -> Result<Vec<(String, String)>> {
    let session_store = SessionStore::load(root).unwrap_or_default();
    let proc_store = ProcessStore::load().unwrap_or_default();

    let session = if let Some(id) = session_id {
        session_store
            .find_by_any_id(id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", id))?
    } else {
        session_store
            .latest()
            .ok_or_else(|| anyhow::anyhow!("No sessions found"))?
    };

    let proc_entry = proc_store
        .processes
        .iter()
        .filter(|e| e.session_id.as_deref() == Some(&session.session_id))
        .max_by(|a, b| a.started_at.cmp(&b.started_at));

    let mut vars: Vec<(String, String)> = Vec::new();

    vars.push(("ZAG_SESSION_ID".to_string(), session.session_id.clone()));

    if let Some(ref name) = session.name {
        vars.push(("ZAG_SESSION_NAME".to_string(), name.clone()));
    }

    if let Some(pe) = proc_entry {
        vars.push(("ZAG_PROCESS_ID".to_string(), pe.id.clone()));
    }

    vars.push(("ZAG_PROVIDER".to_string(), session.provider.clone()));

    if !session.model.is_empty() {
        vars.push(("ZAG_MODEL".to_string(), session.model.clone()));
    }

    if !session.worktree_path.is_empty() {
        vars.push(("ZAG_ROOT".to_string(), session.worktree_path.clone()));
    }

    Ok(vars)
}

/// Run the env command.
pub fn run_env(session_id: Option<&str>, shell: bool, root: Option<&str>) -> Result<()> {
    let vars = get_env_vars(session_id, root)?;

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
