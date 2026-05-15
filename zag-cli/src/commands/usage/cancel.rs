use anyhow::{Result, bail};
use zag_orch::usage_resume_store;

/// `zag usage cancel <incident_id>` — write a tombstone so the next
/// rehydration pass skips the incident and `zag usage list` stops
/// showing it. Does NOT abort an in-process timer in another running
/// relay; if you also need that, kill the relay.
pub(crate) fn run(incident_id: &str, json: bool, root: Option<&str>) -> Result<()> {
    let pending = usage_resume_store::list_pending(root)?;
    if !pending.iter().any(|p| p.incident_id == incident_id) {
        bail!(
            "No pending resume found for incident {incident_id}. Use \
             `zag usage list` to see in-flight resumes."
        );
    }
    usage_resume_store::record_cancel(root, incident_id)?;
    if json {
        let value = serde_json::json!({
            "action": "cancel",
            "incident_id": incident_id,
            "status": "ok",
        });
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        println!("Cancelled pending resume {incident_id}.");
    }
    Ok(())
}
