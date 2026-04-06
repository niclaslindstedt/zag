use super::*;

#[test]
fn test_build_plan_prompt_basic() {
    let prompt = build_plan_prompt("Add authentication", None);
    assert!(prompt.contains("Add authentication"));
    assert!(prompt.contains("Implementation Steps"));
    assert!(!prompt.contains("Additional Instructions"));
}

#[test]
fn test_build_plan_prompt_with_instructions() {
    let prompt = build_plan_prompt("Add auth", Some("Use JWT tokens"));
    assert!(prompt.contains("Add auth"));
    assert!(prompt.contains("Use JWT tokens"));
    assert!(prompt.contains("Additional Instructions"));
}

#[test]
fn test_resolve_output_path_file() {
    let path = resolve_output_path("plans/my-plan.md");
    assert_eq!(path, PathBuf::from("plans/my-plan.md"));
}

#[test]
fn test_resolve_output_path_directory() {
    let path = resolve_output_path("plans");
    assert!(path.starts_with("plans"));
    assert!(
        path.file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("plan-")
    );
    assert!(path.file_name().unwrap().to_str().unwrap().ends_with(".md"));
}

#[test]
fn test_validate_output_path_no_env() {
    // Without ZAG_USER_HOME_DIR, all paths are allowed
    // Note: we don't modify env vars here since that's unsafe in Rust 2024
    // and tests run in parallel. Instead we test the logic by verifying
    // that when the var is unset (default in test), validation always passes.
    assert!(validate_output_path(Path::new("/tmp/anything")).is_ok());
    assert!(validate_output_path(Path::new("/etc/plan.md")).is_ok());
}
