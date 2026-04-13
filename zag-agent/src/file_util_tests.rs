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
    // No stale temp files should remain
    let temps: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(temps.is_empty(), "temp files should be cleaned up");
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

#[test]
fn test_unique_tmp_paths_are_distinct() {
    let path = Path::new("/tmp/logs/index.json");
    let a = unique_tmp_path(path);
    let b = unique_tmp_path(path);
    assert_ne!(a, b, "each call should produce a unique temp path");
    assert!(a.to_string_lossy().ends_with(".tmp"));
    assert!(a.to_string_lossy().starts_with("/tmp/logs/.index.json."));
}

#[test]
fn test_atomic_write_concurrent_writers_all_succeed() {
    let dir = temp_dir();
    let target = dir.join("index.json");
    let num_threads = 16;
    let writes_per_thread = 50;

    let barrier = std::sync::Arc::new(std::sync::Barrier::new(num_threads));
    let errors = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));

    let handles: Vec<_> = (0..num_threads)
        .map(|t| {
            let target = target.clone();
            let barrier = barrier.clone();
            let errors = errors.clone();
            std::thread::spawn(move || {
                barrier.wait();
                for i in 0..writes_per_thread {
                    let content = format!(r#"{{"thread":{},"iter":{}}}"#, t, i);
                    if let Err(e) = atomic_write_str(&target, &content) {
                        errors.lock().unwrap().push(format!(
                            "thread {} iter {}: {}",
                            t, i, e
                        ));
                    }
                }
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    let errs = errors.lock().unwrap();
    assert!(errs.is_empty(), "concurrent writes failed: {:?}", *errs);

    // The file should contain valid JSON from one of the writers.
    let final_content = std::fs::read_to_string(&target).unwrap();
    assert!(final_content.contains("thread"));

    // No temp files should remain.
    let temps: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(temps.is_empty(), "stale temp files: {:?}", temps);

    std::fs::remove_dir_all(&dir).ok();
}
