use anyhow::Result;
use zag_agent::session;

#[allow(clippy::too_many_arguments)]
pub(crate) fn run(
    provider: Option<String>,
    limit: Option<usize>,
    global: bool,
    name: Option<String>,
    tag: Option<String>,
    parent: Option<String>,
    json: bool,
    root: Option<&str>,
) -> Result<()> {
    let store = if global {
        session::SessionStore::load_all()?
    } else {
        session::SessionStore::load(root)?
    };
    let mut sessions = store.list();
    if let Some(ref p) = provider {
        sessions.retain(|s| s.provider == *p);
    }
    if let Some(ref n) = name {
        let n_lower = n.to_lowercase();
        sessions.retain(|s| {
            s.name
                .as_ref()
                .map(|sn| sn.to_lowercase().contains(&n_lower))
                .unwrap_or(false)
        });
    }
    if let Some(ref t) = tag {
        let t_lower = t.to_lowercase();
        sessions.retain(|s| s.tags.iter().any(|st| st.to_lowercase() == t_lower));
    }
    if let Some(ref parent_id) = parent {
        let proc_store = zag_agent::process_store::ProcessStore::load().unwrap_or_default();
        let child_session_ids: std::collections::HashSet<String> = proc_store
            .processes
            .iter()
            .filter(|e| {
                e.parent_session_id.as_deref() == Some(parent_id)
                    || e.parent_process_id.as_deref() == Some(parent_id)
            })
            .filter_map(|e| e.session_id.clone())
            .collect();
        sessions.retain(|s| child_session_ids.contains(&s.session_id));
    }
    if let Some(n) = limit {
        sessions.truncate(n);
    }
    if json {
        println!("{}", serde_json::to_string(&sessions)?);
        return Ok(());
    }
    if sessions.is_empty() {
        println!("No sessions found.");
        return Ok(());
    }
    println!(
        "{:<38} {:<20} {:<10} {:<12} CREATED",
        "SESSION ID", "NAME", "PROVIDER", "MODEL"
    );
    println!("{}", "-".repeat(110));
    for s in &sessions {
        let name_display = s
            .name
            .as_deref()
            .map(|n| {
                if n.len() > 18 {
                    format!("{}…", &n[..17])
                } else {
                    n.to_string()
                }
            })
            .unwrap_or_else(|| "-".to_string());
        println!(
            "{:<38} {:<20} {:<10} {:<12} {}",
            s.session_id, name_display, s.provider, s.model, s.created_at
        );
    }
    Ok(())
}
