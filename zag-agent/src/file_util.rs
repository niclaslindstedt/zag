//! Atomic file write utilities.
//!
//! Provides helpers that write to a temporary file and then rename,
//! ensuring the target file is never left in a partially-written state.

use anyhow::{Context, Result};
use std::path::Path;

/// Write `content` to `path` atomically.
///
/// Writes to a sibling `.tmp` file first, then renames. On Unix,
/// `rename()` is atomic within the same filesystem, so the target file
/// is either the old version or the new one — never a partial write.
pub fn atomic_write(path: &Path, content: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, content)
        .with_context(|| format!("Failed to write temp file: {}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, path).with_context(|| {
        format!(
            "Failed to rename {} -> {}",
            tmp_path.display(),
            path.display()
        )
    })?;
    Ok(())
}

/// Convenience wrapper: atomically write a `&str` to `path`.
pub fn atomic_write_str(path: &Path, content: &str) -> Result<()> {
    atomic_write(path, content.as_bytes())
}

#[cfg(test)]
#[path = "file_util_tests.rs"]
mod tests;
