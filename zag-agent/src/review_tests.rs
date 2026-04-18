use super::*;

#[test]
fn review_template_has_placeholders() {
    assert!(REVIEW_TEMPLATE.contains("{DIFF}"));
    assert!(REVIEW_TEMPLATE.contains("{TITLE_SECTION}"));
    assert!(REVIEW_TEMPLATE.contains("{PROMPT}"));
}

#[test]
fn build_review_prompt_injects_all_sections() {
    let rendered = build_review_prompt(
        "diff content here",
        Some("Security audit"),
        Some("focus on auth"),
    );
    assert!(rendered.contains("diff content here"));
    assert!(rendered.contains("## Review Title"));
    assert!(rendered.contains("Security audit"));
    assert!(rendered.contains("focus on auth"));
    assert!(!rendered.contains("{DIFF}"));
    assert!(!rendered.contains("{TITLE_SECTION}"));
    assert!(!rendered.contains("{PROMPT}"));
}

#[test]
fn build_review_prompt_without_optional_fields() {
    let rendered = build_review_prompt("d", None, None);
    assert!(rendered.contains("d"));
    assert!(!rendered.contains("## Review Title"));
    assert!(!rendered.contains("{DIFF}"));
}

#[tokio::test]
async fn run_review_requires_a_target() {
    let result = run_review(ReviewParams {
        provider: "claude".to_string(),
        uncommitted: false,
        base: None,
        commit: None,
        ..ReviewParams::default()
    })
    .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Review requires"));
}
