use super::*;
use chrono::Duration;
use tempfile::TempDir;

fn make_pending(incident_id: &str, when: DateTime<Utc>, dir: &TempDir) -> PendingResume {
    PendingResume {
        incident_id: incident_id.to_string(),
        session_id: format!("session-{incident_id}"),
        provider: "claude".to_string(),
        model: None,
        root: None,
        when,
        message: "Continue".to_string(),
        attempt: 1,
        log_path: dir.path().join("session.jsonl"),
    }
}

#[test]
fn empty_store_yields_no_pending() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("scheduled_resumes.jsonl");
    assert_eq!(list_pending_at(&path).unwrap(), Vec::new());
}

#[test]
fn schedule_then_list_returns_record() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("scheduled_resumes.jsonl");
    let when = Utc::now() + Duration::seconds(60);
    let p = make_pending("inc-1", when, &dir);
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .unwrap();
    writeln!(
        f,
        "{}",
        serde_json::to_string(&Record::Schedule(p.clone())).unwrap()
    )
    .unwrap();
    let pending = list_pending_at(&path).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].incident_id, "inc-1");
}

#[test]
fn complete_tombstone_removes_from_pending() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("scheduled_resumes.jsonl");
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .unwrap();
    let p = make_pending("inc-2", Utc::now() + Duration::seconds(60), &dir);
    writeln!(
        f,
        "{}",
        serde_json::to_string(&Record::Schedule(p)).unwrap()
    )
    .unwrap();
    writeln!(
        f,
        "{}",
        serde_json::to_string(&Record::Complete {
            incident_id: "inc-2".to_string(),
            status: "resumed".to_string(),
            error: None,
        })
        .unwrap()
    )
    .unwrap();
    assert!(list_pending_at(&path).unwrap().is_empty());
}

#[test]
fn cancel_tombstone_removes_from_pending() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("scheduled_resumes.jsonl");
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .unwrap();
    let p = make_pending("inc-3", Utc::now() + Duration::seconds(60), &dir);
    writeln!(
        f,
        "{}",
        serde_json::to_string(&Record::Schedule(p)).unwrap()
    )
    .unwrap();
    writeln!(
        f,
        "{}",
        serde_json::to_string(&Record::Cancel {
            incident_id: "inc-3".to_string(),
        })
        .unwrap()
    )
    .unwrap();
    assert!(list_pending_at(&path).unwrap().is_empty());
}

#[test]
fn pending_sorted_by_wake_time() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("scheduled_resumes.jsonl");
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .unwrap();
    let now = Utc::now();
    let a = make_pending("late", now + Duration::seconds(600), &dir);
    let b = make_pending("early", now + Duration::seconds(60), &dir);
    let c = make_pending("middle", now + Duration::seconds(300), &dir);
    for p in [&a, &b, &c] {
        writeln!(
            f,
            "{}",
            serde_json::to_string(&Record::Schedule(p.clone())).unwrap()
        )
        .unwrap();
    }
    let pending = list_pending_at(&path).unwrap();
    let ids: Vec<&str> = pending.iter().map(|p| p.incident_id.as_str()).collect();
    assert_eq!(ids, ["early", "middle", "late"]);
}

#[test]
fn malformed_line_is_skipped_gracefully() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("scheduled_resumes.jsonl");
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .unwrap();
    let p = make_pending("good", Utc::now() + Duration::seconds(60), &dir);
    writeln!(
        f,
        "{}",
        serde_json::to_string(&Record::Schedule(p)).unwrap()
    )
    .unwrap();
    writeln!(f, "this is not json").unwrap();
    f.write_all(b"\n").unwrap();
    let pending = list_pending_at(&path).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].incident_id, "good");
}

#[test]
fn last_completion_wins_when_duplicated() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("scheduled_resumes.jsonl");
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .unwrap();
    let p = make_pending("dup", Utc::now() + Duration::seconds(60), &dir);
    writeln!(
        f,
        "{}",
        serde_json::to_string(&Record::Schedule(p)).unwrap()
    )
    .unwrap();
    // Two completes for the same id should still result in no pending.
    for status in ["resumed", "failed"] {
        writeln!(
            f,
            "{}",
            serde_json::to_string(&Record::Complete {
                incident_id: "dup".to_string(),
                status: status.to_string(),
                error: None,
            })
            .unwrap()
        )
        .unwrap();
    }
    assert!(list_pending_at(&path).unwrap().is_empty());
}
