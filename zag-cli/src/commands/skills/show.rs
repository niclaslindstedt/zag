use anyhow::Result;
use zag::skills;

pub(crate) fn run(name: &str, json: bool) -> Result<()> {
    let skill = skills::get_skill(name)?;
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
    Ok(())
}
