//! Atomic file write utilities.
//!
//! Provides helpers that write to a temporary file and then rename,
//! ensuring the target file is never left in a partially-written state.
//! Temp files use a unique name per call (PID + counter) so concurrent
//! writers targeting the same path do not collide.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Monotonic counter to ensure unique temp filenames within a process.
static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Build a unique sibling temp path for the given target, e.g.
/// `logs/index.json` → `logs/.index.json.12345.0.tmp`.
fn unique_tmp_path(path: &Path) -> PathBuf {
    let counter = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("zag-atomic");
    let tmp_name = format!(".{file_name}.{pid}.{counter}.tmp");
    match path.parent() {
        Some(parent) => parent.join(tmp_name),
        None => PathBuf::from(tmp_name),
    }
}

/// Write `content` to `path` atomically.
///
/// Writes to a uniquely-named sibling temp file first, then renames.
/// On Unix, `rename()` is atomic within the same filesystem, so the
/// target file is either the old version or the new one — never a
/// partial write. The unique temp name prevents concurrent writers
/// from clobbering each other's temp files.
pub fn atomic_write(path: &Path, content: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }
    let tmp_path = unique_tmp_path(path);
    std::fs::write(&tmp_path, content)
        .with_context(|| format!("Failed to write temp file: {}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, path).with_context(|| {
        // Clean up the temp file on rename failure.
        let _ = std::fs::remove_file(&tmp_path);
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
