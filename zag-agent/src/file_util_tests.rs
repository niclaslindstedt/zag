use super::*;
use std::path::PathBuf;

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("zag-test-file-util-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_atomic_write_creates_file() {
    let dir = temp_dir();
    let path = dir.join("test.json");
    atomic_write_str(&path, r#"{"hello":"world"}"#).unwrap();
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        r#"{"hello":"world"}"#
    );
    // Temp file should not remain
    assert!(!path.with_extension("tmp").exists());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_atomic_write_overwrites_existing() {
    let dir = temp_dir();
    let path = dir.join("data.json");
    std::fs::write(&path, "old content").unwrap();
    atomic_write_str(&path, "new content").unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "new content");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_atomic_write_creates_parent_dirs() {
    let dir = temp_dir();
    let path = dir.join("nested").join("deep").join("file.json");
    atomic_write_str(&path, "nested content").unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "nested content");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_atomic_write_bytes() {
    let dir = temp_dir();
    let path = dir.join("bytes.bin");
    let data: Vec<u8> = vec![0x00, 0x01, 0xFF, 0xFE];
    atomic_write(&path, &data).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), data);
    std::fs::remove_dir_all(&dir).ok();
}
