use anyhow::{Result, bail};

/// Embedded manpages.
const MAN_ZAG: &str = include_str!("../man/zag.md");
const MAN_RUN: &str = include_str!("../man/run.md");
const MAN_EXEC: &str = include_str!("../man/exec.md");
const MAN_REVIEW: &str = include_str!("../man/review.md");
const MAN_CONFIG: &str = include_str!("../man/config.md");
const MAN_SESSION: &str = include_str!("../man/session.md");
const MAN_CAPABILITY: &str = include_str!("../man/capability.md");
const MAN_LISTEN: &str = include_str!("../man/listen.md");
const MAN_MAN: &str = include_str!("../man/man.md");
const MAN_SKILLS: &str = include_str!("../man/skills.md");
const MAN_MCP: &str = include_str!("../man/mcp.md");
const MAN_PS: &str = include_str!("../man/ps.md");
const MAN_SEARCH: &str = include_str!("../man/search.md");
const MAN_INPUT: &str = include_str!("../man/input.md");

/// AI-oriented reference document for `--help-agent`.
pub(crate) const HELP_AGENT: &str = include_str!("../man/help-agent.md");

/// Print a manpage to stdout.
pub(crate) fn print_manpage(command: Option<&str>) -> Result<()> {
    let content = match command {
        None | Some("zag") => MAN_ZAG,
        Some("run") => MAN_RUN,
        Some("exec") => MAN_EXEC,
        Some("review") => MAN_REVIEW,
        Some("config") => MAN_CONFIG,
        Some("session") => MAN_SESSION,
        Some("capability") => MAN_CAPABILITY,
        Some("listen") => MAN_LISTEN,
        Some("man") => MAN_MAN,
        Some("skills") => MAN_SKILLS,
        Some("mcp") => MAN_MCP,
        Some("ps") => MAN_PS,
        Some("search") => MAN_SEARCH,
        Some("input") => MAN_INPUT,
        Some(other) => bail!(
            "No manual entry for '{}'. Available: run, exec, review, config, session, capability, listen, man, skills, mcp, ps, search, input",
            other
        ),
    };
    print!("{}", content);
    Ok(())
}

#[cfg(test)]
#[path = "manpage_tests.rs"]
mod tests;
