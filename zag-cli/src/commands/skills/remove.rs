use anyhow::Result;
use zag_agent::skills;

pub(crate) fn run(name: &str) -> Result<()> {
    skills::remove_skill(name)?;
    println!(
        "\x1b[32m✓\x1b[0m Removed skill '{}' and its provider symlinks.",
        name
    );
    Ok(())
}
