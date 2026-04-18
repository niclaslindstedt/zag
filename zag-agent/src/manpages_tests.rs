use super::*;

#[test]
fn help_agent_is_non_empty() {
    assert!(HELP_AGENT.starts_with("# zag CLI"));
    assert!(HELP_AGENT.contains("zag exec"));
}

#[test]
fn zag_manpage_lists_commands() {
    assert!(ZAG.contains("run"));
    assert!(ZAG.contains("--help-agent"));
}

#[test]
fn manpage_none_returns_top_level() {
    assert_eq!(manpage(None), Some(ZAG));
    assert_eq!(manpage(Some("")), Some(ZAG));
    assert_eq!(manpage(Some("zag")), Some(ZAG));
}

#[test]
fn manpage_known_commands() {
    assert_eq!(manpage(Some("run")), Some(RUN));
    assert_eq!(manpage(Some("exec")), Some(EXEC));
    assert_eq!(manpage(Some("review")), Some(REVIEW));
    assert_eq!(manpage(Some("plan")), Some(PLAN));
    assert_eq!(manpage(Some("orchestration")), Some(ORCHESTRATION));
}

#[test]
fn disconnect_aliases_to_connect() {
    assert_eq!(manpage(Some("disconnect")), Some(CONNECT));
}

#[test]
fn unknown_command_returns_none() {
    assert!(manpage(Some("nonexistent")).is_none());
    assert!(manpage(Some("frobnicate")).is_none());
}

#[test]
fn manpage_names_covers_every_entry() {
    for name in manpage_names() {
        assert!(
            manpage(Some(name)).is_some(),
            "manpage({name:?}) returned None but is listed in MANPAGE_NAMES"
        );
    }
}

#[test]
fn every_const_appears_in_names() {
    // Guard against adding a new const without listing it.
    let names: std::collections::HashSet<&str> = MANPAGE_NAMES.iter().copied().collect();
    for required in [
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
        "user",
        "orchestration",
    ] {
        assert!(
            names.contains(required),
            "{required} missing from MANPAGE_NAMES"
        );
    }
}
