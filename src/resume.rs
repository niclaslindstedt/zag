//! Session resume resolution: discovering and resolving sessions for `--resume` / `--continue`.

use crate::claude;
use crate::codex;
use crate::copilot;
use crate::gemini;
use crate::session;
use crate::worktree;
use log::debug;
use std::path::PathBuf;

/// A discovered provider-native session (not yet in the session store).
pub struct DiscoveredSession {
    pub provider: String,
    pub provider_session_id: String,
    pub workspace_path: Option<String>,
    pub discovery_source: String,
}

/// A resolved resume target: a session entry plus how it was matched.
#[derive(Clone)]
pub struct ResumeTarget {
    pub entry: session::SessionEntry,
    pub matched_by_wrapper_id: bool,
}

fn home_dir() -> Option<PathBuf> {
    dirs::home_dir()
}

fn is_wrapper_worktree_path(path: &str) -> bool {
    let Some(root) = home_dir().map(|h| h.join(".agent").join("worktrees")) else {
        return false;
    };
    std::path::Path::new(path).starts_with(root)
}

fn worktree_name_from_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default()
}

pub fn current_workspace(root: Option<&str>) -> String {
    if let Some(root) = root {
        root.to_string()
    } else if let Ok(repo_root) = worktree::git_repo_root(None) {
        repo_root.to_string_lossy().to_string()
    } else {
        std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    }
}

/// Detect a provider-native session by scanning known provider file locations.
pub fn detect_provider_session(session_id: &str) -> Option<DiscoveredSession> {
    if let Some(claude_projects) = claude::projects_dir()
        && let Ok(projects) = std::fs::read_dir(&claude_projects)
    {
        for project in projects.flatten() {
            let candidate = project.path().join(format!("{}.jsonl", session_id));
            if candidate.exists() {
                let workspace_path = std::fs::read_to_string(&candidate)
                    .ok()
                    .and_then(|content| {
                        content.lines().find_map(|line| {
                            serde_json::from_str::<serde_json::Value>(line)
                                .ok()
                                .and_then(|json| {
                                    json.get("cwd")
                                        .and_then(|value| value.as_str())
                                        .map(str::to_string)
                                })
                        })
                    });
                return Some(DiscoveredSession {
                    provider: "claude".to_string(),
                    provider_session_id: session_id.to_string(),
                    workspace_path,
                    discovery_source: candidate.to_string_lossy().to_string(),
                });
            }
        }
    }

    let codex_history = codex::history_path();
    if let Ok(content) = std::fs::read_to_string(&codex_history) {
        let needle = format!("\"session_id\":\"{}\"", session_id);
        if content.contains(&needle) {
            return Some(DiscoveredSession {
                provider: "codex".to_string(),
                provider_session_id: session_id.to_string(),
                workspace_path: None,
                discovery_source: codex_history.to_string_lossy().to_string(),
            });
        }
    }

    if let Some(gemini_tmp) = gemini::tmp_dir()
        && let Ok(projects) = std::fs::read_dir(&gemini_tmp)
    {
        for project in projects.flatten() {
            let chats = project.path().join("chats");
            if let Ok(files) = std::fs::read_dir(&chats) {
                for file in files.flatten() {
                    if let Ok(content) = std::fs::read_to_string(file.path()) {
                        let needle = format!("\"sessionId\": \"{}\"", session_id);
                        if content.contains(&needle) {
                            return Some(DiscoveredSession {
                                provider: "gemini".to_string(),
                                provider_session_id: session_id.to_string(),
                                workspace_path: None,
                                discovery_source: file.path().to_string_lossy().to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    let copilot_dir = copilot::session_state_dir().join(session_id);
    if copilot_dir.join("events.jsonl").exists() {
        return Some(DiscoveredSession {
            provider: "copilot".to_string(),
            provider_session_id: session_id.to_string(),
            workspace_path: None,
            discovery_source: copilot_dir.to_string_lossy().to_string(),
        });
    }

    None
}

/// Cache a discovered session into the session store and return the entry.
pub fn cache_discovered_session(
    discovered: &DiscoveredSession,
    root: Option<&str>,
) -> session::SessionEntry {
    let existing_model = session::SessionStore::load(root)
        .unwrap_or_default()
        .find_by_any_id(&discovered.provider_session_id)
        .map(|e| e.model.clone())
        .unwrap_or_default();

    let workspace_path = discovered
        .workspace_path
        .clone()
        .unwrap_or_else(|| current_workspace(root));
    let is_worktree = is_wrapper_worktree_path(&workspace_path);
    let entry = session::SessionEntry {
        session_id: discovered.provider_session_id.clone(),
        provider: discovered.provider.clone(),
        model: existing_model,
        worktree_path: workspace_path.clone(),
        worktree_name: if is_worktree {
            worktree_name_from_path(&workspace_path)
        } else {
            String::new()
        },
        created_at: chrono::Utc::now().to_rfc3339(),
        provider_session_id: Some(discovered.provider_session_id.clone()),
        sandbox_name: None,
        is_worktree,
        discovered: true,
        discovery_source: Some(discovered.discovery_source.clone()),
        log_path: None,
        log_completeness: "partial".to_string(),
    };

    let mut store = session::SessionStore::load(root).unwrap_or_default();
    store.add(entry.clone());
    if let Err(e) = store.save(root) {
        log::warn!("Failed to cache discovered session: {}", e);
    }

    entry
}

/// Resolve a resume target by session ID (wrapper or provider-native).
pub fn resolve_resume_target(requested_id: &str, root: Option<&str>) -> Option<ResumeTarget> {
    let store = session::SessionStore::load(root).unwrap_or_default();
    if let Some(entry) = store.find_by_any_id(requested_id) {
        debug!(
            "Found session in store: id={}, provider={}, model='{}'",
            entry.session_id, entry.provider, entry.model
        );
        return Some(ResumeTarget {
            entry: entry.clone(),
            matched_by_wrapper_id: store.find_by_session_id(requested_id).is_some(),
        });
    }

    debug!(
        "Session {} not in store, trying provider discovery",
        requested_id
    );
    let discovered = detect_provider_session(requested_id)?;
    let entry = cache_discovered_session(&discovered, root);
    debug!(
        "Discovered session: provider={}, model='{}'",
        entry.provider, entry.model
    );
    Some(ResumeTarget {
        entry,
        matched_by_wrapper_id: false,
    })
}

/// Resolve --continue (latest session).
pub fn resolve_continue_target(root: Option<&str>) -> Option<ResumeTarget> {
    let store = session::SessionStore::load(root).unwrap_or_default();
    store.latest().map(|entry| ResumeTarget {
        entry: entry.clone(),
        matched_by_wrapper_id: true,
    })
}

/// Read the native session ID and optional cwd from a Claude `.jsonl` session file.
pub fn read_claude_session_metadata(path: &std::path::Path) -> Option<(String, Option<String>)> {
    let file = std::fs::File::open(path).ok()?;
    let reader = std::io::BufReader::new(file);
    use std::io::BufRead;
    let mut session_id = None;
    let mut cwd = None;
    for line in reader.lines().take(10) {
        let line = line.ok()?;
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
            if session_id.is_none() {
                session_id = value
                    .get("sessionId")
                    .or_else(|| value.get("session_id"))
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
            }
            if cwd.is_none() {
                cwd = value
                    .get("cwd")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
            }
            if session_id.is_some() && cwd.is_some() {
                break;
            }
        }
    }
    session_id.map(|sid| (sid, cwd))
}

/// Discover the provider-native session ID after an agent session completes.
pub fn discover_provider_session_id(
    provider: &str,
    _wrapper_session_id: Option<&str>,
    _root: Option<&str>,
    _workspace_path: Option<&str>,
) -> Option<String> {
    match provider {
        "claude" => {
            let projects_dir = claude::projects_dir()?;
            let workspace = _workspace_path;
            let entries = std::fs::read_dir(&projects_dir).ok()?;
            let mut newest: Option<(std::time::SystemTime, String)> = None;
            for project in entries.flatten() {
                let files = match std::fs::read_dir(project.path()) {
                    Ok(files) => files,
                    Err(_) => continue,
                };
                for file in files.flatten() {
                    let path = file.path();
                    if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
                        continue;
                    }
                    let metadata = match file.metadata() {
                        Ok(m) => m,
                        Err(_) => continue,
                    };
                    let modified = match metadata.modified() {
                        Ok(m) => m,
                        Err(_) => continue,
                    };
                    if newest.as_ref().map(|(t, _)| modified > *t).unwrap_or(true)
                        && let Some((sid, file_cwd)) = read_claude_session_metadata(&path)
                    {
                        if let Some(ws) = workspace
                            && file_cwd.as_deref() != Some(ws)
                        {
                            continue;
                        }
                        newest = Some((modified, sid));
                    }
                }
            }
            newest.map(|(_, sid)| sid)
        }
        "codex" => {
            let history = codex::history_path();
            let content = std::fs::read_to_string(history).ok()?;
            content
                .lines()
                .rev()
                .find_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
                .and_then(|json| {
                    json.get("session_id")
                        .and_then(|value| value.as_str())
                        .map(str::to_string)
                })
        }
        "gemini" => {
            let gemini_tmp = gemini::tmp_dir()?;
            let mut newest: Option<(std::time::SystemTime, String)> = None;
            let projects = std::fs::read_dir(gemini_tmp).ok()?;
            for project in projects.flatten() {
                let chats = project.path().join("chats");
                let files = match std::fs::read_dir(chats) {
                    Ok(files) => files,
                    Err(_) => continue,
                };
                for file in files.flatten() {
                    let path = file.path();
                    let metadata = match file.metadata() {
                        Ok(metadata) => metadata,
                        Err(_) => continue,
                    };
                    let modified = match metadata.modified() {
                        Ok(modified) => modified,
                        Err(_) => continue,
                    };
                    let content = match std::fs::read_to_string(path) {
                        Ok(content) => content,
                        Err(_) => continue,
                    };
                    let session_id = match serde_json::from_str::<serde_json::Value>(&content)
                        .ok()
                        .and_then(|json| {
                            json.get("sessionId")
                                .and_then(|value| value.as_str())
                                .map(str::to_string)
                        }) {
                        Some(session_id) => session_id,
                        None => continue,
                    };
                    if newest
                        .as_ref()
                        .map(|(current, _)| modified > *current)
                        .unwrap_or(true)
                    {
                        newest = Some((modified, session_id));
                    }
                }
            }
            newest.map(|(_, session_id)| session_id)
        }
        "copilot" => {
            let chat_sessions = copilot::session_state_dir();
            let mut newest: Option<(std::time::SystemTime, String)> = None;
            let entries = std::fs::read_dir(chat_sessions).ok()?;
            for entry in entries.flatten() {
                let events_path = entry.path().join("events.jsonl");
                if !events_path.exists() {
                    continue;
                }
                let metadata = match std::fs::metadata(&events_path) {
                    Ok(metadata) => metadata,
                    Err(_) => continue,
                };
                let modified = match metadata.modified() {
                    Ok(modified) => modified,
                    Err(_) => continue,
                };
                let session_id = entry.file_name().to_string_lossy().to_string();
                if newest
                    .as_ref()
                    .map(|(current, _)| modified > *current)
                    .unwrap_or(true)
                {
                    newest = Some((modified, session_id));
                }
            }
            newest.map(|(_, session_id)| session_id)
        }
        _ => None,
    }
}
