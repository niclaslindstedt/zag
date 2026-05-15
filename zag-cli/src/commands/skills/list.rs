use anyhow::Result;
use zag_agent::skills;

pub(crate) fn run(json: bool) -> Result<()> {
    let skill_list = skills::list_skills()?;
    if json {
        println!("{}", serde_json::to_string(&skill_list)?);
        return Ok(());
    }
    if skill_list.is_empty() {
        println!("No skills found in {}", skills::skills_dir().display());
        println!("Use 'zag skills add <name>' to create one.");
        return Ok(());
    }
    println!("{:<20} {:<50} PATH", "NAME", "DESCRIPTION");
    println!("{}", "-".repeat(100));
    for skill in &skill_list {
        println!(
            "{:<20} {:<50} {}",
            skill.name,
            if skill.description.len() > 48 {
                format!("{}...", &skill.description[..48])
            } else {
                skill.description.clone()
            },
            skill.dir.display()
        );
    }
    Ok(())
}
