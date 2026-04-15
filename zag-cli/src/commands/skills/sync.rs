use anyhow::Result;
use zag_agent::skills;

pub(crate) fn run(provider: Option<String>) -> Result<()> {
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
            println!("  {p} does not support native skills (skipped)");
        }
    }
    Ok(())
}
