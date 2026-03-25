/// Provider-agnostic skill management.
///
/// Skills are stored at `~/.zag/skills/<skill-name>/` and symlinked into each
/// provider's native skill location when running agents.
///
/// Providers with native skill support (Claude, Gemini, Copilot) get directory symlinks.
/// Providers without (Codex, Ollama) get skill content injected into the system prompt.
#[cfg(test)]
#[path = "skills_tests.rs"]
mod tests;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const SKILL_PREFIX: &str = "zag-";
const IMPORT_METADATA_FILE: &str = ".import-metadata.json";

/// Metadata written when a skill is imported from a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportMetadata {
    pub source_provider: String,
    pub source_hash: String,
    pub imported_at: String,
}

/// Compute SHA-256 hash of a skill's SKILL.md file.
pub fn hash_skill_md(skill_dir: &Path) -> Result<String> {
    let content = fs::read(skill_dir.join("SKILL.md"))
        .with_context(|| format!("Failed to read SKILL.md in {}", skill_dir.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(hex::encode(hasher.finalize()))
}

/// Write import metadata to a skill directory.
pub fn write_import_metadata(skill_dir: &Path, provider: &str, hash: &str) -> Result<()> {
    let meta = ImportMetadata {
        source_provider: provider.to_string(),
        source_hash: hash.to_string(),
        imported_at: chrono::Utc::now().to_rfc3339(),
    };
    let path = skill_dir.join(IMPORT_METADATA_FILE);
    let json = serde_json::to_string_pretty(&meta)?;
    fs::write(&path, json).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

/// Read import metadata from a skill directory. Returns None if missing or invalid.
pub fn read_import_metadata(skill_dir: &Path) -> Option<ImportMetadata> {
    let path = skill_dir.join(IMPORT_METADATA_FILE);
    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Check if a path is a real directory (not a symlink).
fn is_real_dir(path: &Path) -> bool {
    path.exists()
        && !path
            .symlink_metadata()
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false)
}

/// A parsed agent skill.
#[derive(Debug, Clone, Serialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    /// Markdown body (everything after the frontmatter)
    pub body: String,
    /// Path to the skill directory
    pub dir: PathBuf,
}

#[derive(Debug, Deserialize)]
struct SkillFrontmatter {
    name: String,
    #[serde(default)]
    description: String,
}

/// Returns `~/.zag/skills/`.
pub fn skills_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".zag")
        .join("skills")
}

/// Returns the provider's native skill directory, or `None` if the provider has no native support.
///
/// - Claude: `~/.claude/skills/`
/// - Gemini: `~/.gemini/skills/` (also supports `~/.agents/skills/`)
/// - Copilot: `~/.copilot/skills/`
/// - Codex: `~/.agents/skills/` (agentskills.io standard)
pub fn provider_skills_dir(provider: &str) -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    match provider {
        "claude" => Some(home.join(".claude").join("skills")),
        "gemini" => Some(home.join(".gemini").join("skills")),
        "copilot" => Some(home.join(".copilot").join("skills")),
        "codex" => Some(home.join(".agents").join("skills")),
        _ => None,
    }
}

/// Parse a skill from its directory. Reads `<dir>/SKILL.md`.
pub fn parse_skill(dir: &Path) -> Result<Skill> {
    let skill_file = dir.join("SKILL.md");
    let content = fs::read_to_string(&skill_file)
        .with_context(|| format!("Failed to read {}", skill_file.display()))?;

    let (frontmatter, body) = split_frontmatter(&content)?;

    let meta: SkillFrontmatter = serde_yaml::from_str(&frontmatter).with_context(|| {
        format!(
            "Failed to parse YAML frontmatter in {}",
            skill_file.display()
        )
    })?;

    Ok(Skill {
        name: meta.name,
        description: meta.description,
        body: body.trim().to_string(),
        dir: dir.to_path_buf(),
    })
}

/// Split YAML frontmatter from markdown body.
/// Returns `(frontmatter, body)`. Frontmatter is between the first two `---` lines.
fn split_frontmatter(content: &str) -> Result<(String, String)> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() || lines[0].trim() != "---" {
        bail!("SKILL.md must start with --- (YAML frontmatter)");
    }

    let end = lines
        .iter()
        .skip(1)
        .position(|l| l.trim() == "---")
        .context("SKILL.md frontmatter not closed with ---")?;

    let frontmatter = lines[1..=end].join("\n");
    let body = lines[end + 2..].join("\n");

    Ok((frontmatter, body))
}

/// Load all skills from `~/.zag/skills/`.
/// Silently skips directories without a valid `SKILL.md`.
pub fn load_all_skills() -> Result<Vec<Skill>> {
    let dir = skills_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut skills = Vec::new();
    for entry in fs::read_dir(&dir)
        .with_context(|| format!("Failed to read skills directory {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        match parse_skill(&path) {
            Ok(skill) => skills.push(skill),
            Err(e) => {
                log::warn!("Skipping skill at {}: {}", path.display(), e);
            }
        }
    }

    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(skills)
}

/// Sync skills for a provider that supports native skills (Claude, Gemini, Copilot).
/// Creates `<provider_skills_dir>/zag-<name>` → `~/.zag/skills/<name>` symlinks.
/// Removes stale symlinks for skills that no longer exist.
/// Returns the number of skills skipped because the provider already has them natively.
pub fn sync_skills_for_provider(provider: &str, skills: &[Skill]) -> Result<usize> {
    let Some(target_dir) = provider_skills_dir(provider) else {
        return Ok(0);
    };

    fs::create_dir_all(&target_dir).with_context(|| {
        format!(
            "Failed to create {} skills directory {}",
            provider,
            target_dir.display()
        )
    })?;

    // Create/update symlinks for current skills
    let mut skipped = 0usize;
    for skill in skills {
        // Skip if the provider already has this skill natively (not via our symlink)
        let native_path = target_dir.join(&skill.name);
        if is_real_dir(&native_path) {
            let native_hash = hash_skill_md(&native_path).ok();
            let import_meta = read_import_metadata(&skill.dir);
            if let (Some(nh), Some(meta)) = (&native_hash, &import_meta)
                && *nh != meta.source_hash
            {
                log::warn!(
                    "Skill '{}' has diverged from native {} version",
                    skill.name,
                    provider
                );
            }
            skipped += 1;
            continue;
        }

        let link_name = format!("{}{}", SKILL_PREFIX, skill.name);
        let link_path = target_dir.join(&link_name);
        let target = &skill.dir;

        // Remove existing entry if it's wrong or stale
        if link_path.exists() || link_path.symlink_metadata().is_ok() {
            let is_correct_symlink = link_path.symlink_metadata().is_ok()
                && fs::read_link(&link_path)
                    .map(|t| t == *target)
                    .unwrap_or(false);
            if is_correct_symlink {
                continue;
            }
            if link_path.is_dir() && link_path.symlink_metadata().is_err() {
                // Real directory — don't touch it
                log::warn!(
                    "Skipping {}: a real directory already exists there",
                    link_path.display()
                );
                continue;
            }
            fs::remove_file(&link_path)
                .or_else(|_| remove_symlink_dir(&link_path))
                .with_context(|| format!("Failed to remove stale entry {}", link_path.display()))?;
        }

        std::os::unix::fs::symlink(target, &link_path).with_context(|| {
            format!(
                "Failed to create symlink {} -> {}",
                link_path.display(),
                target.display()
            )
        })?;

        log::debug!(
            "Linked skill '{}' for {}: {} -> {}",
            skill.name,
            provider,
            link_path.display(),
            target.display()
        );
    }

    // Remove stale symlinks (our agent-* prefixed ones whose source no longer exists)
    let skill_names: std::collections::HashSet<String> =
        skills.iter().map(|s| s.name.clone()).collect();

    if let Ok(entries) = fs::read_dir(&target_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();

            if !name.starts_with(SKILL_PREFIX) {
                continue;
            }

            // Only touch symlinks we created
            if path.symlink_metadata().is_err() {
                continue;
            }
            if path.is_dir() && path.symlink_metadata().is_ok() {
                // It's a symlink (to a dir)
            } else if !path
                .symlink_metadata()
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false)
            {
                continue;
            }

            let skill_name = name.trim_start_matches(SKILL_PREFIX);
            // Remove if the skill no longer exists OR if the provider now has it natively
            let should_remove =
                !skill_names.contains(skill_name) || is_real_dir(&target_dir.join(skill_name));
            if should_remove {
                let _ = fs::remove_file(&path).or_else(|_| remove_symlink_dir(&path));
                log::debug!("Removed stale skill symlink: {}", path.display());
            }
        }
    }

    Ok(skipped)
}

#[cfg(unix)]
fn remove_symlink_dir(path: &Path) -> std::io::Result<()> {
    // On Unix, symlinks to directories are removed with remove_file
    fs::remove_file(path)
}

#[cfg(not(unix))]
fn remove_symlink_dir(path: &Path) -> std::io::Result<()> {
    fs::remove_dir(path)
}

/// Format skills as system prompt content (for providers without native skill support).
pub fn format_skills_for_system_prompt(skills: &[Skill]) -> String {
    if skills.is_empty() {
        return String::new();
    }

    let mut out = String::from("\n\n## Agent Skills\n\nThe following skills are available:\n");
    for skill in skills {
        out.push_str(&format!("\n### Skill: {}\n", skill.name));
        if !skill.description.is_empty() {
            out.push_str(&format!("_{}_\n\n", skill.description));
        }
        if !skill.body.is_empty() {
            out.push_str(&skill.body);
            out.push('\n');
        }
    }
    out
}

/// Orchestrate skill setup for the given provider.
///
/// - Providers with native skills (claude, gemini, copilot): create directory symlinks.
/// - Providers without (codex, ollama): append skill content to system_prompt.
pub fn setup_skills(provider: &str, system_prompt: &mut Option<String>) -> Result<()> {
    let skills = load_all_skills()?;
    if skills.is_empty() {
        return Ok(());
    }

    if provider_skills_dir(provider).is_some() {
        // Native skills support — symlink skill directories
        let skipped = sync_skills_for_provider(provider, &skills)?;
        let synced = skills.len() - skipped;
        if skipped > 0 {
            log::info!(
                "Synced {} skill(s) for {} (skipped {} native duplicate(s))",
                synced,
                provider,
                skipped
            );
        } else {
            log::info!("Synced {} skill(s) for {}", synced, provider);
        }
    } else {
        // No native skills — inject into system prompt
        let injected = format_skills_for_system_prompt(&skills);
        match system_prompt {
            Some(sp) => sp.push_str(&injected),
            None => *system_prompt = Some(injected),
        }
        log::debug!(
            "Injected {} skill(s) into system prompt for {}",
            skills.len(),
            provider
        );
    }

    Ok(())
}

/// List all skills (alias for load_all_skills, used by the subcommand).
pub fn list_skills() -> Result<Vec<Skill>> {
    load_all_skills()
}

/// Get a single skill by name from `~/.zag/skills/<name>/`.
pub fn get_skill(name: &str) -> Result<Skill> {
    let dir = skills_dir().join(name);
    if !dir.exists() {
        bail!("Skill '{}' not found at {}", name, dir.display());
    }
    parse_skill(&dir)
}

/// Create a new skill skeleton at `~/.zag/skills/<name>/SKILL.md`.
/// Returns the path to the new skill directory.
pub fn add_skill(name: &str, description: &str) -> Result<PathBuf> {
    let dir = skills_dir().join(name);
    if dir.exists() {
        bail!("Skill '{}' already exists at {}", name, dir.display());
    }
    fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create skill directory {}", dir.display()))?;

    let skill_md = dir.join("SKILL.md");
    let content = format!(
        "---\nname: {}\ndescription: {}\n---\n\n# {}\n\nDescribe what this skill does here.\n",
        name, description, name
    );
    fs::write(&skill_md, &content)
        .with_context(|| format!("Failed to write {}", skill_md.display()))?;

    Ok(dir)
}

/// Remove a skill and all its provider symlinks.
pub fn remove_skill(name: &str) -> Result<()> {
    let dir = skills_dir().join(name);
    if !dir.exists() {
        bail!("Skill '{}' not found at {}", name, dir.display());
    }

    // Remove provider symlinks first
    for provider in &["claude", "gemini", "copilot", "codex"] {
        if let Some(provider_dir) = provider_skills_dir(provider) {
            let link = provider_dir.join(format!("{}{}", SKILL_PREFIX, name));
            if link.symlink_metadata().is_ok() {
                let _ = fs::remove_file(&link).or_else(|_| remove_symlink_dir(&link));
                log::debug!("Removed {} symlink: {}", provider, link.display());
            }
        }
    }

    // Remove the skill directory
    fs::remove_dir_all(&dir)
        .with_context(|| format!("Failed to remove skill directory {}", dir.display()))?;

    Ok(())
}

/// Import skills from a provider's native skill directory into `~/.zag/skills/`.
/// Skips directories already prefixed with `agent-` (our own symlinks).
/// Returns names of imported skills.
pub fn import_skills(from_provider: &str) -> Result<Vec<String>> {
    let Some(source_dir) = provider_skills_dir(from_provider) else {
        bail!(
            "Provider '{}' does not have a native skill directory",
            from_provider
        );
    };

    if !source_dir.exists() {
        bail!(
            "No skill directory found for '{}' at {}",
            from_provider,
            source_dir.display()
        );
    }

    let dest_dir = skills_dir();
    fs::create_dir_all(&dest_dir)?;

    let mut imported = Vec::new();

    for entry in fs::read_dir(&source_dir)
        .with_context(|| format!("Failed to read {}", source_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        // Skip our own agent-* symlinks
        if name.starts_with(SKILL_PREFIX) {
            continue;
        }

        // Only handle directories
        if !path.is_dir() {
            continue;
        }

        // Skip if SKILL.md is missing
        if !path.join("SKILL.md").exists() {
            continue;
        }

        let dest = dest_dir.join(name.as_ref());
        if dest.exists() {
            // Backfill metadata for previously imported skills that lack it
            if read_import_metadata(&dest).is_none()
                && let Ok(source_hash) = hash_skill_md(&path)
            {
                let _ = write_import_metadata(&dest, from_provider, &source_hash);
                log::info!("Backfilled import metadata for skill '{}'", name);
            }
            log::debug!("Skipping '{}': already exists in ~/.zag/skills/", name);
            continue;
        }

        copy_dir_all(&path, &dest).with_context(|| format!("Failed to copy skill '{}'", name))?;

        // Write import metadata with hash of the source SKILL.md
        if let Ok(source_hash) = hash_skill_md(&path) {
            let _ = write_import_metadata(&dest, from_provider, &source_hash);
        }

        imported.push(name.to_string());
    }

    Ok(imported)
}

/// Recursively copy a directory.
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst_path)?;
        } else {
            fs::copy(entry.path(), dst_path)?;
        }
    }
    Ok(())
}
