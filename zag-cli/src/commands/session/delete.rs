use anyhow::{Result, bail};
use zag::session;

pub(crate) fn run(id: &str, json: bool, root: Option<&str>) -> Result<()> {
    let mut store = session::SessionStore::load(root)?;
    if store.get(id).is_none() {
        bail!("Session not found: {}", id);
    }
    store.remove(id);
    store.save(root)?;
    if json {
        println!(r#"{{"deleted":"{}"}}"#, id);
    } else {
        println!("Deleted session: {}", id);
    }
    Ok(())
}
