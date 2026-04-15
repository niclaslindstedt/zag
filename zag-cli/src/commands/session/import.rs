use anyhow::Result;

use crate::session_log;

pub(crate) fn run(root: Option<&str>) -> Result<()> {
    let imported = session_log::run_default_backfill(root)?;
    println!("Imported {imported} historical session log(s)");
    Ok(())
}
