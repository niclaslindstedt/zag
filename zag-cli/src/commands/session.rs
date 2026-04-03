use anyhow::{Result, bail};
use zag::session;

use crate::cli::SessionCommand;
use crate::session_log;

pub(crate) fn run_session(command: SessionCommand, json: bool, root: Option<&str>) -> Result<()> {
    match command {
        SessionCommand::List {
            provider,
            limit,
            global,
            name,
            tag,
            parent,
        } => {
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
                // Cross-reference with process store to find child sessions
                let proc_store = zag::process_store::ProcessStore::load().unwrap_or_default();
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
        }
        SessionCommand::Show { id } => {
            let store = session::SessionStore::load(root)?;
            match store.get(&id) {
                Some(info) => {
                    if json {
                        println!("{}", serde_json::to_string(&info)?);
                        return Ok(());
                    }
                    println!("Session ID:          {}", info.session_id);
                    println!("Provider:            {}", info.provider);
                    println!("Model:               {}", info.model);
                    println!("Created:             {}", info.created_at);
                    if let Some(ref name) = info.name {
                        println!("Name:                {}", name);
                    }
                    if let Some(ref desc) = info.description {
                        println!("Description:         {}", desc);
                    }
                    if !info.tags.is_empty() {
                        println!("Tags:                {}", info.tags.join(", "));
                    }
                    if let Some(ref pid) = info.provider_session_id {
                        println!("Provider Session ID: {}", pid);
                    }
                    if let Some(ref wp) = info.worktree_path {
                        println!("Worktree:            {}", wp);
                    }
                    if let Some(ref sb) = info.sandbox_name {
                        println!("Sandbox:             {}", sb);
                    }
                    println!("Log Completeness:    {}", info.log_completeness);
                }
                None => {
                    bail!("Session not found: {}", id);
                }
            }
        }
        SessionCommand::Import => {
            let imported = session_log::run_default_backfill(root)?;
            println!("Imported {} historical session log(s)", imported);
        }
        SessionCommand::Delete { id } => {
            let mut store = session::SessionStore::load(root)?;
            if store.get(&id).is_none() {
                bail!("Session not found: {}", id);
            }
            store.remove(&id);
            store.save(root)?;
            if json {
                println!(r#"{{"deleted":"{}"}}"#, id);
            } else {
                println!("Deleted session: {}", id);
            }
        }
        SessionCommand::Update {
            id,
            name,
            description,
            tags,
            clear_tags,
        } => {
            let mut store = session::SessionStore::load(root)?;
            let entry = store.sessions.iter_mut().find(|e| e.session_id == id);
            let entry = match entry {
                Some(e) => e,
                None => bail!("Session not found: {}", id),
            };
            if name.is_some() {
                entry.name = name;
            }
            if description.is_some() {
                entry.description = description;
            }
            if clear_tags {
                entry.tags.clear();
            }
            if !tags.is_empty() {
                entry.tags.extend(tags);
            }
            let updated = session::SessionInfo::from(&*entry);
            store.save(root)?;
            if json {
                println!("{}", serde_json::to_string(&updated)?);
            } else {
                println!("Updated session: {}", id);
            }
        }
    }
    Ok(())
}
