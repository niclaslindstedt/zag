use super::*;

#[test]
fn plan_template_has_placeholders() {
    assert!(PLAN_TEMPLATE.contains("{GOAL}"));
    assert!(PLAN_TEMPLATE.contains("{CONTEXT_SECTION}"));
    assert!(PLAN_TEMPLATE.contains("{PROMPT}"));
}

#[test]
fn build_plan_prompt_basic() {
    let prompt = build_plan_prompt("Add authentication", None);
    assert!(prompt.contains("Add authentication"));
    assert!(!prompt.contains("Additional Instructions"));
    assert!(!prompt.contains("{GOAL}"));
}

#[test]
fn build_plan_prompt_with_instructions() {
    let prompt = build_plan_prompt("Add auth", Some("Use JWT tokens"));
    assert!(prompt.contains("Add auth"));
    assert!(prompt.contains("Use JWT tokens"));
    assert!(prompt.contains("Additional Instructions"));
}

#[test]
fn resolve_output_path_file() {
    let path = resolve_output_path("plans/my-plan.md");
    assert_eq!(path, PathBuf::from("plans/my-plan.md"));
}

#[test]
fn resolve_output_path_directory() {
    let path = resolve_output_path("plans");
    assert!(path.starts_with("plans"));
    let name = path.file_name().unwrap().to_str().unwrap();
    assert!(name.starts_with("plan-"));
    assert!(name.ends_with(".md"));
}

#[test]
fn validate_output_path_no_env() {
    // Without ZAG_USER_HOME_DIR, all paths are allowed.
    assert!(validate_output_path(Path::new("/tmp/anything")).is_ok());
    assert!(validate_output_path(Path::new("/etc/plan.md")).is_ok());
}
