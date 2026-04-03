use anyhow::Result;
use zag::skills;

use crate::cli::SkillsCommand;

pub(crate) fn run_skills(command: SkillsCommand, json: bool) -> Result<()> {
    match command {
        SkillsCommand::List => {
            let skill_list = skills::list_skills()?;
            if json {
                println!("{}", serde_json::to_string(&skill_list)?);
                return Ok(());
            }
            if skill_list.is_empty() {
                println!("No skills found in {}", skills::skills_dir().display());
                println!("Use 'agent skills add <name>' to create one.");
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
        }
        SkillsCommand::Show { name } => {
            let skill = skills::get_skill(&name)?;
            if json {
                println!("{}", serde_json::to_string(&skill)?);
                return Ok(());
            }
            println!("Name:        {}", skill.name);
            println!("Description: {}", skill.description);
            println!("Path:        {}", skill.dir.display());
            if !skill.body.is_empty() {
                println!();
                println!("{}", skill.body);
            }
        }
        SkillsCommand::Add { name, description } => {
            let description = description.unwrap_or_default();
            let dir = skills::add_skill(&name, &description)?;
            println!(
                "\x1b[32m✓\x1b[0m Created skill '{}' at {}",
                name,
                dir.display()
            );
            println!(
                "Edit {} to add your skill content.",
                dir.join("SKILL.md").display()
            );
        }
        SkillsCommand::Remove { name } => {
            skills::remove_skill(&name)?;
            println!(
                "\x1b[32m✓\x1b[0m Removed skill '{}' and its provider symlinks.",
                name
            );
        }
        SkillsCommand::Sync { provider } => {
            let skill_list = skills::load_all_skills()?;
            if skill_list.is_empty() {
                println!("No skills to sync.");
                return Ok(());
            }
            let providers: Vec<&str> = if let Some(ref p) = provider {
                vec![p.as_str()]
            } else {
                vec!["claude", "gemini", "copilot", "codex"]
            };
            for p in providers {
                if skills::provider_skills_dir(p).is_some() {
                    skills::sync_skills_for_provider(p, &skill_list)?;
                    println!(
                        "\x1b[32m✓\x1b[0m Synced {} skill(s) for {}",
                        skill_list.len(),
                        p
                    );
                } else {
                    println!("  {} does not support native skills (skipped)", p);
                }
            }
        }
        SkillsCommand::Import { from } => {
            let imported = skills::import_skills(&from)?;
            if imported.is_empty() {
                println!("No new skills to import from '{}'.", from);
            } else {
                for name in &imported {
                    println!("\x1b[32m✓\x1b[0m Imported skill '{}'", name);
                }
                println!("Imported {} skill(s) from '{}'.", imported.len(), from);
            }
        }
    }
    Ok(())
}
