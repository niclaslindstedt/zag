use super::*;

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
            "manpage for '{}' failed",
            cmd
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
    assert!(MAN_ZAG.contains("# zag"));
    assert!(MAN_RUN.contains("# zag run"));
    assert!(MAN_EXEC.contains("# zag exec"));
    assert!(MAN_REVIEW.contains("# zag review"));
    assert!(MAN_CONFIG.contains("# zag config"));
    assert!(MAN_MAN.contains("# zag man"));
    assert!(MAN_SKILLS.contains("# zag skills"));
    assert!(MAN_MCP.contains("# zag mcp"));
    assert!(MAN_PS.contains("# zag ps"));
    assert!(MAN_SEARCH.contains("# zag search"));
    assert!(MAN_INPUT.contains("# zag input"));
    assert!(MAN_BROADCAST.contains("# zag broadcast"));
}
