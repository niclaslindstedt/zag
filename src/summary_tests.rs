use super::*;

#[test]
fn format_duration_seconds() {
    assert_eq!(format_duration(45.0), "45s");
}

#[test]
fn format_duration_minutes() {
    assert_eq!(format_duration(154.0), "2m 34s");
}

#[test]
fn format_duration_hours() {
    assert_eq!(format_duration(3720.0), "1h 2m");
}
