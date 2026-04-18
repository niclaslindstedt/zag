use super::*;
use zag_agent::manpages;

#[test]
fn test_print_manpage_default() {
    assert!(print_manpage(None).is_ok());
}

#[test]
fn test_print_manpage_zag() {
    assert!(print_manpage(Some("zag")).is_ok());
}

#[test]
fn test_print_manpage_all_commands() {
    for cmd in &[
        "run",
        "exec",
        "review",
        "config",
        "session",
        "capability",
        "listen",
        "man",
        "skills",
        "mcp",
        "ps",
        "search",
        "input",
        "broadcast",
    ] {
        assert!(
            print_manpage(Some(cmd)).is_ok(),
            "manpage for '{cmd}' failed"
        );
    }
}

#[test]
fn test_print_manpage_unknown_command() {
    let result = print_manpage(Some("nonexistent"));
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("No manual entry"));
    assert!(err.contains("nonexistent"));
}

#[test]
fn test_manpage_content_has_headers() {
    assert!(manpages::ZAG.contains("# zag"));
    assert!(manpages::RUN.contains("# zag run"));
    assert!(manpages::EXEC.contains("# zag exec"));
    assert!(manpages::REVIEW.contains("# zag review"));
    assert!(manpages::CONFIG.contains("# zag config"));
    assert!(manpages::MAN.contains("# zag man"));
    assert!(manpages::SKILLS.contains("# zag skills"));
    assert!(manpages::MCP.contains("# zag mcp"));
    assert!(manpages::PS.contains("# zag ps"));
    assert!(manpages::SEARCH.contains("# zag search"));
    assert!(manpages::INPUT.contains("# zag input"));
    assert!(manpages::BROADCAST.contains("# zag broadcast"));
}

#[test]
fn test_help_agent_reexport_matches_library() {
    assert_eq!(HELP_AGENT, manpages::HELP_AGENT);
}
