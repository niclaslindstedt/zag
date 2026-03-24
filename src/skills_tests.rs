use super::*;
use std::fs;
use tempfile::TempDir;

fn make_skill_dir(parent: &Path, name: &str, description: &str, body: &str) -> PathBuf {
    let dir = parent.join(name);
    fs::create_dir_all(&dir).unwrap();
    let content = format!(
        "---\nname: {}\ndescription: {}\n---\n\n{}\n",
        name, description, body
    );
    fs::write(dir.join("SKILL.md"), content).unwrap();
    dir
}

#[test]
fn test_parse_skill_valid() {
    let tmp = TempDir::new().unwrap();
    make_skill_dir(
        tmp.path(),
        "my-skill",
        "Does stuff",
        "# My Skill\n\nHelps you do things.",
    );

    let skill = parse_skill(&tmp.path().join("my-skill")).unwrap();
    assert_eq!(skill.name, "my-skill");
    assert_eq!(skill.description, "Does stuff");
    assert!(skill.body.contains("# My Skill"));
    assert!(skill.body.contains("Helps you do things."));
}

#[test]
fn test_parse_skill_no_frontmatter() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("bad-skill");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("SKILL.md"), "No frontmatter here").unwrap();

    assert!(parse_skill(&dir).is_err());
}

#[test]
fn test_parse_skill_unclosed_frontmatter() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("bad-skill");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("SKILL.md"), "---\nname: test\n").unwrap();

    assert!(parse_skill(&dir).is_err());
}

#[test]
fn test_load_all_skills_empty_dir() {
    let tmp = TempDir::new().unwrap();
    // Simulate empty ~/.agent/skills/ by pointing skills_dir equivalent at tmp
    // Since we can't override skills_dir() easily, test load logic directly
    let skills = load_skills_from(tmp.path()).unwrap();
    assert!(skills.is_empty());
}

#[test]
fn test_load_all_skills_multiple() {
    let tmp = TempDir::new().unwrap();
    make_skill_dir(tmp.path(), "skill-a", "Skill A description", "Body A");
    make_skill_dir(tmp.path(), "skill-b", "Skill B description", "Body B");

    let skills = load_skills_from(tmp.path()).unwrap();
    assert_eq!(skills.len(), 2);
    // Should be sorted by name
    assert_eq!(skills[0].name, "skill-a");
    assert_eq!(skills[1].name, "skill-b");
}

#[test]
fn test_load_all_skills_skips_invalid() {
    let tmp = TempDir::new().unwrap();
    // Valid skill
    make_skill_dir(tmp.path(), "good-skill", "Good", "Body");
    // Invalid: no SKILL.md
    fs::create_dir_all(tmp.path().join("empty-dir")).unwrap();

    let skills = load_skills_from(tmp.path()).unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "good-skill");
}

#[test]
fn test_format_skills_for_system_prompt_empty() {
    let result = format_skills_for_system_prompt(&[]);
    assert!(result.is_empty());
}

#[test]
fn test_format_skills_for_system_prompt_with_skills() {
    let tmp = TempDir::new().unwrap();
    let dir = make_skill_dir(
        tmp.path(),
        "test-skill",
        "Test description",
        "## Instructions\nDo this.",
    );
    let skill = parse_skill(&dir).unwrap();

    let result = format_skills_for_system_prompt(&[skill]);
    assert!(result.contains("## Agent Skills"));
    assert!(result.contains("### Skill: test-skill"));
    assert!(result.contains("Test description"));
    assert!(result.contains("## Instructions"));
}

#[test]
fn test_sync_skills_for_provider_creates_symlinks() {
    let tmp = TempDir::new().unwrap();
    let skills_src = tmp.path().join("agent-skills");
    let provider_skills = tmp.path().join("provider-skills");
    fs::create_dir_all(&skills_src).unwrap();
    fs::create_dir_all(&provider_skills).unwrap();

    make_skill_dir(&skills_src, "my-skill", "A skill", "Body here");
    let skill = parse_skill(&skills_src.join("my-skill")).unwrap();

    sync_skills_for_provider_to(&provider_skills, &[skill]).unwrap();

    let link = provider_skills.join("agent-my-skill");
    assert!(link.symlink_metadata().is_ok(), "symlink should exist");
    assert!(link.is_dir(), "symlink should resolve to a directory");
    assert!(
        link.join("SKILL.md").exists(),
        "SKILL.md should be accessible through symlink"
    );
}

#[test]
fn test_sync_skills_removes_stale_symlinks() {
    let tmp = TempDir::new().unwrap();
    let skills_src = tmp.path().join("agent-skills");
    let provider_skills = tmp.path().join("provider-skills");
    fs::create_dir_all(&skills_src).unwrap();
    fs::create_dir_all(&provider_skills).unwrap();

    // Create a skill, symlink it, then remove the skill and re-sync with empty list
    make_skill_dir(&skills_src, "old-skill", "Old", "Body");
    let skill = parse_skill(&skills_src.join("old-skill")).unwrap();
    sync_skills_for_provider_to(&provider_skills, &[skill]).unwrap();

    let link = provider_skills.join("agent-old-skill");
    assert!(link.symlink_metadata().is_ok());

    // Re-sync with no skills — stale link should be removed
    sync_skills_for_provider_to(&provider_skills, &[]).unwrap();
    assert!(
        !link.symlink_metadata().is_ok(),
        "stale symlink should be removed"
    );
}

#[test]
fn test_add_and_remove_skill() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path().join("agent").join("skills");
    fs::create_dir_all(&base).unwrap();

    // add
    let skill_dir = add_skill_to(&base, "test-skill", "A test skill").unwrap();
    assert!(skill_dir.exists());
    assert!(skill_dir.join("SKILL.md").exists());
    let skill = parse_skill(&skill_dir).unwrap();
    assert_eq!(skill.name, "test-skill");

    // remove
    remove_skill_from(&base, "test-skill", &[]).unwrap();
    assert!(!skill_dir.exists());
}

#[test]
fn test_add_skill_already_exists() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path().join("skills");
    fs::create_dir_all(&base).unwrap();

    add_skill_to(&base, "dupe", "First").unwrap();
    assert!(add_skill_to(&base, "dupe", "Second").is_err());
}

// Internal helpers that accept a base path for testability

pub(crate) fn load_skills_from(dir: &Path) -> Result<Vec<Skill>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut skills = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path.join("SKILL.md").exists() {
            match parse_skill(&path) {
                Ok(s) => skills.push(s),
                Err(e) => log::warn!("Skipping {}: {}", path.display(), e),
            }
        }
    }
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(skills)
}

pub(crate) fn sync_skills_for_provider_to(provider_dir: &Path, skills: &[Skill]) -> Result<()> {
    fs::create_dir_all(provider_dir)?;

    let skill_names: std::collections::HashSet<String> =
        skills.iter().map(|s| s.name.clone()).collect();

    for skill in skills {
        let link_name = format!("{}{}", SKILL_PREFIX, skill.name);
        let link_path = provider_dir.join(&link_name);
        let target = &skill.dir;

        if link_path.symlink_metadata().is_ok() {
            if fs::read_link(&link_path)
                .map(|t| t == *target)
                .unwrap_or(false)
            {
                continue;
            }
            let _ = fs::remove_file(&link_path).or_else(|_| fs::remove_dir(&link_path));
        }

        std::os::unix::fs::symlink(target, &link_path)?;
    }

    // Remove stale
    if let Ok(entries) = fs::read_dir(provider_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            if !name.starts_with(SKILL_PREFIX) {
                continue;
            }
            if path.symlink_metadata().is_err() {
                continue;
            }
            let skill_name = name.trim_start_matches(SKILL_PREFIX);
            if !skill_names.contains(skill_name) {
                let _ = fs::remove_file(&path).or_else(|_| fs::remove_dir(&path));
            }
        }
    }

    Ok(())
}

pub(crate) fn add_skill_to(base: &Path, name: &str, description: &str) -> Result<PathBuf> {
    let dir = base.join(name);
    if dir.exists() {
        bail!("Skill '{}' already exists", name);
    }
    fs::create_dir_all(&dir)?;
    let content = format!(
        "---\nname: {}\ndescription: {}\n---\n\n# {}\n\nDescribe what this skill does here.\n",
        name, description, name
    );
    fs::write(dir.join("SKILL.md"), &content)?;
    Ok(dir)
}

pub(crate) fn remove_skill_from(base: &Path, name: &str, provider_dirs: &[&Path]) -> Result<()> {
    let dir = base.join(name);
    if !dir.exists() {
        bail!("Skill '{}' not found", name);
    }
    for provider_dir in provider_dirs {
        let link = provider_dir.join(format!("{}{}", SKILL_PREFIX, name));
        if link.symlink_metadata().is_ok() {
            let _ = fs::remove_file(&link).or_else(|_| fs::remove_dir(&link));
        }
    }
    fs::remove_dir_all(&dir)?;
    Ok(())
}
