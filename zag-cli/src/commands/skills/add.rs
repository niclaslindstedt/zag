use anyhow::Result;
use zag_agent::skills;

pub(crate) fn run(name: &str, description: Option<String>) -> Result<()> {
    let description = description.unwrap_or_default();
    let dir = skills::add_skill(name, &description)?;
    println!(
        "\x1b[32m✓\x1b[0m Created skill '{}' at {}",
        name,
        dir.display()
    );
    println!(
        "Edit {} to add your skill content.",
        dir.join("SKILL.md").display()
    );
    Ok(())
}
