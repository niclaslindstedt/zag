use anyhow::{Result, bail};
use serde::Serialize;

#[derive(Serialize)]
struct WhoamiInfo {
    session_id: Option<String>,
    session_name: Option<String>,
    process_id: Option<String>,
    pid: u32,
    provider: Option<String>,
    model: Option<String>,
    root: Option<String>,
    parent_session_id: Option<String>,
    parent_process_id: Option<String>,
}

impl WhoamiInfo {
    fn from_env() -> Self {
        Self {
            session_id: std::env::var("ZAG_SESSION_ID").ok(),
            session_name: std::env::var("ZAG_SESSION_NAME").ok(),
            process_id: std::env::var("ZAG_PROCESS_ID").ok(),
            pid: std::process::id(),
            provider: std::env::var("ZAG_PROVIDER").ok(),
            model: std::env::var("ZAG_MODEL").ok(),
            root: std::env::var("ZAG_ROOT").ok(),
            parent_session_id: None,
            parent_process_id: None,
        }
    }

    fn is_inside_session(&self) -> bool {
        self.session_id.is_some() || self.process_id.is_some()
    }

    /// Cross-reference with process store to find parent info.
    fn enrich_from_store(&mut self) {
        let Some(ref proc_id) = self.process_id else {
            return;
        };
        let Ok(store) = zag::process_store::ProcessStore::load() else {
            return;
        };
        if let Some(entry) = store.find(proc_id) {
            self.parent_session_id.clone_from(&entry.parent_session_id);
            self.parent_process_id.clone_from(&entry.parent_process_id);
        }
    }
}

pub(crate) fn run_whoami(json: bool) -> Result<()> {
    let mut info = WhoamiInfo::from_env();

    if !info.is_inside_session() {
        if json {
            println!("{{}}");
            return Ok(());
        }
        bail!("Not running inside a zag session.");
    }

    info.enrich_from_store();

    if json {
        println!("{}", serde_json::to_string(&info)?);
        return Ok(());
    }

    if let Some(ref v) = info.session_id {
        println!("Session ID:        {}", v);
    }
    if let Some(ref v) = info.session_name {
        println!("Session Name:      {}", v);
    }
    if let Some(ref v) = info.process_id {
        println!("Process ID:        {}", v);
    }
    println!("PID:               {}", info.pid);
    if let Some(ref v) = info.provider {
        println!("Provider:          {}", v);
    }
    if let Some(ref v) = info.model {
        println!("Model:             {}", v);
    }
    if let Some(ref v) = info.root {
        println!("Root:              {}", v);
    }
    if let Some(ref v) = info.parent_session_id {
        println!("Parent Session ID: {}", v);
    }
    if let Some(ref v) = info.parent_process_id {
        println!("Parent Process ID: {}", v);
    }

    Ok(())
}
