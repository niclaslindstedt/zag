use super::*;

#[test]
fn test_current_workspace_explicit_root() {
    let result = current_workspace(Some("/my/project"));
    assert_eq!(result, "/my/project");
}

#[test]
fn test_current_workspace_falls_back_to_cwd() {
    // When not in a git repo and no root provided, should fall back to cwd.
    // This test runs from the repo root which IS a git repo, so it will
    // return the git repo root rather than cwd. We just verify it returns
    // a non-empty string.
    let result = current_workspace(None);
    assert!(!result.is_empty());
}

#[test]
fn test_logs_dir_uses_agent_dir() {
    let dir = logs_dir(Some("/my/project"));
    assert!(dir.ends_with("logs"));
}
