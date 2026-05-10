//! Helpers for loading versioned prompt files (§13.5).
//!
//! Every file under `prompts/<name>/<X>_<Y>_<Z>.md` begins with a YAML
//! front-matter block declaring `name`, `description`, and a `version`
//! whose value matches the filename stem. The body is split into
//! `## System` and `## User` sections. The Rust code uses these prompts
//! as raw templates substituted with `{PLACEHOLDER}` tokens, so it needs
//! to load the body without the front matter.
//!
//! [`strip_front_matter`] is a no-allocation slice operation. Pass-through
//! when no front matter is present so callers can safely apply it to any
//! string.

/// Strip a leading YAML front-matter block (`---\n...\n---\n`) from a
/// prompt template, returning the body. If the input has no front matter
/// the original slice is returned unchanged.
pub fn strip_front_matter(template: &str) -> &str {
    let Some(rest) = template.strip_prefix("---\n") else {
        return template;
    };
    let Some(end) = rest.find("\n---") else {
        return template;
    };
    let after_marker = &rest[end + "\n---".len()..];
    after_marker.trim_start_matches(['\n', '\r'])
}

#[cfg(test)]
#[path = "prompts_tests.rs"]
mod tests;
