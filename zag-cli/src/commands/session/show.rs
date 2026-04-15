use anyhow::{Result, bail};
use zag_agent::session;

pub(crate) fn run(id: &str, json: bool, root: Option<&str>) -> Result<()> {
    let store = session::SessionStore::load(root)?;
    match store.get(id) {
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
                println!("Name:                {name}");
            }
            if let Some(ref desc) = info.description {
                println!("Description:         {desc}");
            }
            if !info.tags.is_empty() {
                println!("Tags:                {}", info.tags.join(", "));
            }
            if let Some(ref pid) = info.provider_session_id {
                println!("Provider Session ID: {pid}");
            }
            if let Some(ref wp) = info.worktree_path {
                println!("Worktree:            {wp}");
            }
            if let Some(ref sb) = info.sandbox_name {
                println!("Sandbox:             {sb}");
            }
            println!("Log Completeness:    {}", info.log_completeness);
        }
        None => {
            bail!("Session not found: {id}");
        }
    }
    Ok(())
}
