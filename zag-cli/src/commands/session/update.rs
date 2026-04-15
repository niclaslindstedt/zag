use anyhow::{Result, bail};
use zag_agent::session;

pub(crate) fn run(
    id: &str,
    name: Option<String>,
    description: Option<String>,
    tags: Vec<String>,
    clear_tags: bool,
    json: bool,
    root: Option<&str>,
) -> Result<()> {
    let mut store = session::SessionStore::load(root)?;
    let entry = store.sessions.iter_mut().find(|e| e.session_id == id);
    let entry = match entry {
        Some(e) => e,
        None => bail!("Session not found: {id}"),
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
        println!("Updated session: {id}");
    }
    Ok(())
}
