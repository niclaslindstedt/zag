use super::*;

#[test]
fn test_generate_name_has_prefix() {
    let name = generate_name();
    assert!(name.starts_with("agent-"), "name should start with 'agent-': {}", name);
}

#[test]
fn test_generate_name_has_hex_suffix() {
    let name = generate_name();
    let suffix = &name["agent-".len()..];
    assert_eq!(suffix.len(), 8, "hex suffix should be 8 chars: {}", suffix);
    assert!(
        suffix.chars().all(|c| c.is_ascii_hexdigit()),
        "suffix should be hex: {}",
        suffix
    );
}

#[test]
fn test_generate_name_not_empty() {
    let name = generate_name();
    assert!(!name.is_empty());
    assert!(name.len() > "agent-".len());
}

#[test]
fn test_git_repo_root_in_repo() {
    // We're running inside the agent repo, so this should succeed
    let root = git_repo_root(None).unwrap();
    assert!(root.exists());
    assert!(root.join("Cargo.toml").exists());
}

#[test]
fn test_git_repo_root_with_explicit_dir() {
    let root = git_repo_root(Some(".")).unwrap();
    assert!(root.exists());
}

#[test]
fn test_git_repo_root_outside_repo() {
    let result = git_repo_root(Some("/tmp"));
    assert!(result.is_err());
}
