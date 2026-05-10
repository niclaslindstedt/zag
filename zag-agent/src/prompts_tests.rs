use super::*;

#[test]
fn strip_front_matter_removes_yaml_block() {
    let input = "---\nname: plan\nversion: 1.0.0\n---\n\nBody starts here\n";
    assert_eq!(strip_front_matter(input), "Body starts here\n");
}

#[test]
fn strip_front_matter_handles_no_blank_line_after() {
    let input = "---\nname: x\n---\nBody\n";
    assert_eq!(strip_front_matter(input), "Body\n");
}

#[test]
fn strip_front_matter_passes_through_when_absent() {
    let input = "No front matter here\n";
    assert_eq!(strip_front_matter(input), input);
}

#[test]
fn strip_front_matter_passes_through_when_marker_unterminated() {
    let input = "---\nname: x\nno terminator";
    assert_eq!(strip_front_matter(input), input);
}

#[test]
fn strip_front_matter_preserves_inner_dashes() {
    let input = "---\nname: x\n---\n\nBody with --- inside\n";
    assert_eq!(strip_front_matter(input), "Body with --- inside\n");
}
