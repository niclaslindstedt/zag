use anyhow::Result;
use zag_agent::skills;

pub(crate) fn run(from: &str) -> Result<()> {
    let imported = skills::import_skills(from)?;
    if imported.is_empty() {
        println!("No new skills to import from '{}'.", from);
    } else {
        for name in &imported {
            println!("\x1b[32m✓\x1b[0m Imported skill '{}'", name);
        }
        println!("Imported {} skill(s) from '{}'.", imported.len(), from);
    }
    Ok(())
}
