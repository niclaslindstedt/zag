use anyhow::Result;
use zag_orch::usage_resume_store;

/// `zag usage list` — print all pending auto-resume timers.
pub(crate) fn run(json: bool, root: Option<&str>) -> Result<()> {
    let pending = usage_resume_store::list_pending(root)?;

    if json {
        let value = serde_json::to_string_pretty(&pending)?;
        println!("{value}");
        return Ok(());
    }

    if pending.is_empty() {
        println!("No pending auto-resume timers.");
        return Ok(());
    }

    println!(
        "{:<36}  {:<10}  {:<25}  {:<8}  SESSION",
        "INCIDENT", "PROVIDER", "WAKES AT (UTC)", "ATTEMPT"
    );
    for p in &pending {
        println!(
            "{:<36}  {:<10}  {:<25}  {:<8}  {}",
            p.incident_id,
            p.provider,
            p.when.format("%Y-%m-%d %H:%M:%S Z"),
            p.attempt,
            p.session_id,
        );
    }
    Ok(())
}
