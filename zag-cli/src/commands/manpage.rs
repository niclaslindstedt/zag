use anyhow::{Result, bail};

/// Embedded manpages.
const MAN_ZAG: &str = include_str!("../../man/zag.md");
const MAN_RUN: &str = include_str!("../../man/run.md");
const MAN_EXEC: &str = include_str!("../../man/exec.md");
const MAN_REVIEW: &str = include_str!("../../man/review.md");
const MAN_CONFIG: &str = include_str!("../../man/config.md");
const MAN_SESSION: &str = include_str!("../../man/session.md");
const MAN_CAPABILITY: &str = include_str!("../../man/capability.md");
const MAN_LISTEN: &str = include_str!("../../man/listen.md");
const MAN_MAN: &str = include_str!("../../man/man.md");
const MAN_SKILLS: &str = include_str!("../../man/skills.md");
const MAN_MCP: &str = include_str!("../../man/mcp.md");
const MAN_PS: &str = include_str!("../../man/ps.md");
const MAN_SEARCH: &str = include_str!("../../man/search.md");
const MAN_INPUT: &str = include_str!("../../man/input.md");
const MAN_BROADCAST: &str = include_str!("../../man/broadcast.md");
const MAN_WHOAMI: &str = include_str!("../../man/whoami.md");
const MAN_WAIT: &str = include_str!("../../man/wait.md");
const MAN_SPAWN: &str = include_str!("../../man/spawn.md");
const MAN_STATUS: &str = include_str!("../../man/status.md");
const MAN_COLLECT: &str = include_str!("../../man/collect.md");
const MAN_ENV: &str = include_str!("../../man/env.md");
const MAN_PIPE: &str = include_str!("../../man/pipe.md");
const MAN_EVENTS: &str = include_str!("../../man/events.md");
const MAN_CANCEL: &str = include_str!("../../man/cancel.md");
const MAN_SUMMARY: &str = include_str!("../../man/summary.md");
const MAN_WATCH: &str = include_str!("../../man/watch.md");
const MAN_SUBSCRIBE: &str = include_str!("../../man/subscribe.md");
const MAN_LOG: &str = include_str!("../../man/log.md");
const MAN_OUTPUT: &str = include_str!("../../man/output.md");
const MAN_RETRY: &str = include_str!("../../man/retry.md");
const MAN_GC: &str = include_str!("../../man/gc.md");
const MAN_SERVE: &str = include_str!("../../man/serve.md");
const MAN_CONNECT: &str = include_str!("../../man/connect.md");
const MAN_USER: &str = include_str!("../../man/user.md");
const MAN_DISCOVER: &str = include_str!("../../man/discover.md");
const MAN_PLAN: &str = include_str!("../../man/plan.md");
const MAN_ORCHESTRATION: &str = include_str!("../../man/orchestration.md");

/// AI-oriented reference document for `--help-agent`.
pub(crate) const HELP_AGENT: &str = include_str!("../../man/help-agent.md");

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
        Some("broadcast") => MAN_BROADCAST,
        Some("whoami") => MAN_WHOAMI,
        Some("wait") => MAN_WAIT,
        Some("spawn") => MAN_SPAWN,
        Some("status") => MAN_STATUS,
        Some("collect") => MAN_COLLECT,
        Some("env") => MAN_ENV,
        Some("pipe") => MAN_PIPE,
        Some("events") => MAN_EVENTS,
        Some("cancel") => MAN_CANCEL,
        Some("summary") => MAN_SUMMARY,
        Some("watch") => MAN_WATCH,
        Some("subscribe") => MAN_SUBSCRIBE,
        Some("log") => MAN_LOG,
        Some("output") => MAN_OUTPUT,
        Some("retry") => MAN_RETRY,
        Some("gc") => MAN_GC,
        Some("serve") => MAN_SERVE,
        Some("connect") => MAN_CONNECT,
        Some("disconnect") => MAN_CONNECT,
        Some("user") => MAN_USER,
        Some("discover") => MAN_DISCOVER,
        Some("plan") => MAN_PLAN,
        Some("orchestration") => MAN_ORCHESTRATION,
        Some(other) => bail!(
            "No manual entry for '{}'. Available: run, exec, review, plan, config, session, capability, discover, listen, man, skills, mcp, ps, search, input, broadcast, whoami, wait, spawn, status, collect, env, pipe, events, cancel, summary, watch, subscribe, log, output, retry, gc, serve, connect, disconnect, user, orchestration",
            other
        ),
    };
    print!("{}", content);
    Ok(())
}

#[cfg(test)]
#[path = "manpage_tests.rs"]
mod tests;
