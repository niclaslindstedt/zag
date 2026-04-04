use anyhow::Result;
use log::debug;
use zag_agent::{sandbox, session, worktree};

use crate::resume::current_workspace;

#[cfg(test)]
#[path = "session_setup_tests.rs"]
mod tests;

/// Session metadata for discovery (name, description, tags).
#[derive(Clone, Default)]
pub(crate) struct SessionMetadata {
    pub(crate) name: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) tags: Vec<String>,
}

/// Worktree setup state computed before agent creation.
pub(crate) struct WorktreeSetup {
    pub(crate) is_worktree_session: bool,
    pub(crate) session_id: Option<String>,
    pub(crate) worktree_name: Option<String>,
    pub(crate) effective_root: Option<String>,
    pub(crate) worktree_path: Option<String>,
}

pub(crate) struct PlainSessionSetup {
    pub(crate) session_id: Option<String>,
    pub(crate) workspace_path: Option<String>,
}

/// Sandbox setup state computed before agent creation.
pub(crate) struct SandboxSetup {
    pub(crate) is_sandbox_session: bool,
    pub(crate) sandbox_name: Option<String>,
    pub(crate) session_id: Option<String>,
    pub(crate) workspace: Option<String>,
}

/// Set up worktree session state: generate IDs, create worktree.
/// All providers get the same treatment — worktree at `~/.zag/worktrees/<project>/<name>`.
pub(crate) fn setup_worktree(
    worktree_flag: &Option<Option<String>>,
    is_resume: bool,
    root: &Option<String>,
    show_wrapper: bool,
    session_id: Option<String>,
) -> Result<WorktreeSetup> {
    let is_worktree_session = worktree_flag.is_some() && !is_resume;

    if !is_worktree_session {
        return Ok(WorktreeSetup {
            is_worktree_session: false,
            session_id: None,
            worktree_name: None,
            effective_root: root.clone(),
            worktree_path: None,
        });
    }

    let worktree_name = worktree_flag
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("internal error: worktree flag missing"))?
        .as_deref()
        .map(String::from)
        .unwrap_or_else(worktree::generate_name);

    let repo_root = worktree::git_repo_root(root.as_deref())?;
    let name = &worktree_name;
    let wt_path = worktree::create_worktree(&repo_root, name)?;
    if show_wrapper {
        println!("\x1b[32m✓\x1b[0m Worktree created at {}", wt_path.display());
    }
    let path_str = wt_path.to_string_lossy().to_string();

    Ok(WorktreeSetup {
        is_worktree_session: true,
        session_id,
        worktree_name: Some(worktree_name),
        effective_root: Some(path_str.clone()),
        worktree_path: Some(path_str),
    })
}

/// Set up sandbox session state: generate name, session ID, determine workspace.
pub(crate) fn setup_sandbox(
    sandbox_flag: &Option<Option<String>>,
    is_resume: bool,
    root: &Option<String>,
    session_id: Option<String>,
) -> Result<SandboxSetup> {
    let is_sandbox_session = sandbox_flag.is_some() && !is_resume;

    if !is_sandbox_session {
        return Ok(SandboxSetup {
            is_sandbox_session: false,
            sandbox_name: None,
            session_id: None,
            workspace: None,
        });
    }

    let sandbox_name = Some(
        sandbox_flag
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("internal error: sandbox flag missing"))?
            .as_deref()
            .map(String::from)
            .unwrap_or_else(sandbox::generate_name),
    );

    // Determine workspace: root flag > git repo root > current dir
    let workspace = current_workspace(root.as_deref());

    Ok(SandboxSetup {
        is_sandbox_session: true,
        sandbox_name,
        session_id,
        workspace: Some(workspace),
    })
}

pub(crate) fn setup_plain_session(
    is_new_interactive: bool,
    root: &Option<String>,
    explicit_session: &Option<String>,
) -> PlainSessionSetup {
    // If an explicit --session was provided, always use it
    if let Some(session_id) = explicit_session {
        return PlainSessionSetup {
            session_id: Some(session_id.clone()),
            workspace_path: Some(current_workspace(root.as_deref())),
        };
    }

    if !is_new_interactive {
        return PlainSessionSetup {
            session_id: None,
            workspace_path: None,
        };
    }

    PlainSessionSetup {
        session_id: Some(uuid::Uuid::new_v4().to_string()),
        workspace_path: Some(current_workspace(root.as_deref())),
    }
}

/// Save the session-worktree/sandbox mapping to disk.
pub(crate) fn save_session_mapping(
    plain: &PlainSessionSetup,
    wt: &WorktreeSetup,
    sb: &SandboxSetup,
    provider: &str,
    model: &str,
    root: Option<&str>,
    metadata: &SessionMetadata,
) {
    if plain.session_id.is_some() && !wt.is_worktree_session && !sb.is_sandbox_session {
        let mut store = session::SessionStore::load(root).unwrap_or_default();
        store.add(session::SessionEntry {
            session_id: plain.session_id.clone().unwrap_or_default(),
            provider: provider.to_string(),
            model: model.to_string(),
            worktree_path: plain.workspace_path.clone().unwrap_or_default(),
            worktree_name: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            provider_session_id: None,
            sandbox_name: None,
            is_worktree: false,
            discovered: false,
            discovery_source: None,
            log_path: None,
            log_completeness: "partial".to_string(),
            name: metadata.name.clone(),
            description: metadata.description.clone(),
            tags: metadata.tags.clone(),
            dependencies: vec![],
            retried_from: None,
            interactive: false,
        });
        if let Err(e) = store.save(root) {
            log::warn!("Failed to save session mapping: {}", e);
        }
        debug!(
            "Saved plain session mapping: id={}, model='{}'",
            plain.session_id.as_deref().unwrap_or(""),
            model
        );
    }

    // Save worktree session mapping
    if let (Some(sid), Some(wt_path), Some(wt_name)) =
        (&wt.session_id, &wt.worktree_path, &wt.worktree_name)
    {
        let mut store = session::SessionStore::load(root).unwrap_or_default();
        store.add(session::SessionEntry {
            session_id: sid.clone(),
            provider: provider.to_string(),
            model: model.to_string(),
            worktree_path: wt_path.clone(),
            worktree_name: wt_name.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            provider_session_id: None,
            sandbox_name: None,
            is_worktree: true,
            discovered: false,
            discovery_source: None,
            log_path: None,
            log_completeness: "partial".to_string(),
            name: metadata.name.clone(),
            description: metadata.description.clone(),
            tags: metadata.tags.clone(),
            dependencies: vec![],
            retried_from: None,
            interactive: false,
        });
        if let Err(e) = store.save(root) {
            log::warn!("Failed to save session mapping: {}", e);
        }
        debug!("Saved session mapping: {} -> {}", sid, wt_path);
    }

    // Save sandbox session mapping
    if let (Some(sid), Some(sandbox_name)) = (&sb.session_id, &sb.sandbox_name) {
        let workspace = sb.workspace.clone().unwrap_or_default();
        let mut store = session::SessionStore::load(root).unwrap_or_default();
        store.add(session::SessionEntry {
            session_id: sid.clone(),
            provider: provider.to_string(),
            model: model.to_string(),
            worktree_path: workspace.clone(),
            worktree_name: sandbox_name.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            provider_session_id: None,
            sandbox_name: Some(sandbox_name.clone()),
            is_worktree: false,
            discovered: false,
            discovery_source: None,
            log_path: None,
            log_completeness: "partial".to_string(),
            name: metadata.name.clone(),
            description: metadata.description.clone(),
            tags: metadata.tags.clone(),
            dependencies: vec![],
            retried_from: None,
            interactive: false,
        });
        if let Err(e) = store.save(root) {
            log::warn!("Failed to save sandbox session mapping: {}", e);
        }
        debug!("Saved sandbox session mapping: {} -> {}", sid, sandbox_name);
    }
}

pub(crate) fn update_provider_session_id(
    wrapper_session_id: Option<&str>,
    provider_session_id: Option<String>,
    root: Option<&str>,
) {
    let (Some(wrapper_session_id), Some(provider_session_id)) =
        (wrapper_session_id, provider_session_id)
    else {
        return;
    };

    let mut store = session::SessionStore::load(root).unwrap_or_default();
    store.set_provider_session_id(wrapper_session_id, provider_session_id);
    if let Err(e) = store.save(root) {
        log::warn!("Failed to update provider session id: {}", e);
    }
}

pub(crate) fn update_session_log_metadata(
    session_id: Option<&str>,
    log_path: Option<String>,
    completeness: &str,
    root: Option<&str>,
) {
    let Some(session_id) = session_id else {
        return;
    };
    let mut store = session::SessionStore::load(root).unwrap_or_default();
    if let Some(entry) = store
        .sessions
        .iter_mut()
        .find(|entry| entry.session_id == session_id)
    {
        entry.log_path = log_path;
        entry.log_completeness = completeness.to_string();
        let _ = store.save(root);
    }
}
