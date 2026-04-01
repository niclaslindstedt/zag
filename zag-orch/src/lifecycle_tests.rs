use super::*;

#[test]
fn test_events_dir() {
    let dir = events_dir();
    assert!(dir.to_string_lossy().contains("events"));
}
