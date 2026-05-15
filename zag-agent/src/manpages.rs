//! Built-in manpages and the `--help-agent` reference document.
//!
//! Markdown sources live in `zag-agent/man/` and are embedded at compile
//! time via [`include_str!`]. The CLI's `zag man <command>` and
//! `zag --help-agent` subcommands read from this module, and library
//! callers can use the same content programmatically.
//!
//! # Example
//!
//! ```no_run
//! use zag_agent::manpages;
//!
//! // Full AI-oriented reference
//! let help = manpages::HELP_AGENT;
//! println!("{help}");
//!
//! // Per-command manpage
//! if let Some(md) = manpages::manpage(Some("review")) {
//!     println!("{md}");
//! }
//!
//! // Enumerate everything that has a manpage
//! for name in manpages::manpage_names() {
//!     println!("{name}");
//! }
//! ```

/// Top-level CLI overview.
pub const ZAG: &str = include_str!("../man/zag.md");
/// AI-oriented CLI reference, printed by `zag --help-agent`.
pub const HELP_AGENT: &str = include_str!("../man/help-agent.md");
pub const MAN: &str = include_str!("../man/man.md");
pub const RUN: &str = include_str!("../man/run.md");
pub const EXEC: &str = include_str!("../man/exec.md");
pub const REVIEW: &str = include_str!("../man/review.md");
pub const PLAN: &str = include_str!("../man/plan.md");
pub const CONFIG: &str = include_str!("../man/config.md");
pub const SESSION: &str = include_str!("../man/session.md");
pub const CAPABILITY: &str = include_str!("../man/capability.md");
pub const DISCOVER: &str = include_str!("../man/discover.md");
pub const LISTEN: &str = include_str!("../man/listen.md");
pub const SKILLS: &str = include_str!("../man/skills.md");
pub const MCP: &str = include_str!("../man/mcp.md");
pub const PS: &str = include_str!("../man/ps.md");
pub const SEARCH: &str = include_str!("../man/search.md");
pub const INPUT: &str = include_str!("../man/input.md");
pub const BROADCAST: &str = include_str!("../man/broadcast.md");
pub const WHOAMI: &str = include_str!("../man/whoami.md");
pub const WAIT: &str = include_str!("../man/wait.md");
pub const SPAWN: &str = include_str!("../man/spawn.md");
pub const STATUS: &str = include_str!("../man/status.md");
pub const COLLECT: &str = include_str!("../man/collect.md");
pub const ENV: &str = include_str!("../man/env.md");
pub const PIPE: &str = include_str!("../man/pipe.md");
pub const EVENTS: &str = include_str!("../man/events.md");
pub const CANCEL: &str = include_str!("../man/cancel.md");
pub const SUMMARY: &str = include_str!("../man/summary.md");
pub const WATCH: &str = include_str!("../man/watch.md");
pub const SUBSCRIBE: &str = include_str!("../man/subscribe.md");
pub const LOG: &str = include_str!("../man/log.md");
pub const OUTPUT: &str = include_str!("../man/output.md");
pub const RETRY: &str = include_str!("../man/retry.md");
pub const GC: &str = include_str!("../man/gc.md");
pub const SERVE: &str = include_str!("../man/serve.md");
pub const CONNECT: &str = include_str!("../man/connect.md");
pub const USER: &str = include_str!("../man/user.md");
pub const USAGE: &str = include_str!("../man/usage.md");
pub const ORCHESTRATION: &str = include_str!("../man/orchestration.md");

/// Names of every manpage accepted by [`manpage`], in a stable order suited
/// for user-facing listings.
pub const MANPAGE_NAMES: &[&str] = &[
    "zag",
    "run",
    "exec",
    "review",
    "plan",
    "config",
    "session",
    "capability",
    "discover",
    "listen",
    "man",
    "skills",
    "mcp",
    "ps",
    "search",
    "input",
    "broadcast",
    "whoami",
    "wait",
    "spawn",
    "status",
    "collect",
    "env",
    "pipe",
    "events",
    "cancel",
    "summary",
    "watch",
    "subscribe",
    "log",
    "output",
    "retry",
    "gc",
    "serve",
    "connect",
    "disconnect",
    "user",
    "usage",
    "orchestration",
];

/// Return the manpage content for the given command name, or `None` if the
/// name is not recognised. `None` / empty / `"zag"` all return the top-level
/// [`ZAG`] manpage. `"disconnect"` aliases to [`CONNECT`].
pub fn manpage(command: Option<&str>) -> Option<&'static str> {
    Some(match command.unwrap_or("zag") {
        "" | "zag" => ZAG,
        "run" => RUN,
        "exec" => EXEC,
        "review" => REVIEW,
        "plan" => PLAN,
        "config" => CONFIG,
        "session" => SESSION,
        "capability" => CAPABILITY,
        "discover" => DISCOVER,
        "listen" => LISTEN,
        "man" => MAN,
        "skills" => SKILLS,
        "mcp" => MCP,
        "ps" => PS,
        "search" => SEARCH,
        "input" => INPUT,
        "broadcast" => BROADCAST,
        "whoami" => WHOAMI,
        "wait" => WAIT,
        "spawn" => SPAWN,
        "status" => STATUS,
        "collect" => COLLECT,
        "env" => ENV,
        "pipe" => PIPE,
        "events" => EVENTS,
        "cancel" => CANCEL,
        "summary" => SUMMARY,
        "watch" => WATCH,
        "subscribe" => SUBSCRIBE,
        "log" => LOG,
        "output" => OUTPUT,
        "retry" => RETRY,
        "gc" => GC,
        "serve" => SERVE,
        "connect" | "disconnect" => CONNECT,
        "user" => USER,
        "usage" => USAGE,
        "orchestration" => ORCHESTRATION,
        _ => return None,
    })
}

/// Names of every command for which [`manpage`] returns `Some`.
pub fn manpage_names() -> &'static [&'static str] {
    MANPAGE_NAMES
}

#[cfg(test)]
#[path = "manpages_tests.rs"]
mod tests;
