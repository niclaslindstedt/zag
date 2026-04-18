//! Thin shim around [`zag_agent::manpages`] — the markdown content now lives
//! in the `zag-agent` library so that programmatic callers can read
//! `HELP_AGENT` and individual manpages without shelling out to the binary.

use anyhow::{Result, bail};
use zag_agent::manpages;

/// AI-oriented reference document for `--help-agent`. Re-exported for the
/// binary entry point in `main.rs`.
pub(crate) const HELP_AGENT: &str = manpages::HELP_AGENT;

/// Print a manpage to stdout.
pub(crate) fn print_manpage(command: Option<&str>) -> Result<()> {
    match manpages::manpage(command) {
        Some(content) => {
            print!("{content}");
            Ok(())
        }
        None => {
            let names = manpages::manpage_names().join(", ");
            bail!(
                "No manual entry for '{}'. Available: {names}",
                command.unwrap_or("")
            )
        }
    }
}

#[cfg(test)]
#[path = "manpage_tests.rs"]
mod tests;
