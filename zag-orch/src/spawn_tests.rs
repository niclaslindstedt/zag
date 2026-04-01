use super::*;

#[test]
fn test_spawn_logs_dir() {
    let dir = spawn_logs_dir();
    assert!(dir.to_string_lossy().contains("spawn"));
    assert!(dir.to_string_lossy().contains("logs"));
}
