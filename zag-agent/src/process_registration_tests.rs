use super::*;
use crate::process_store::ProcessStore;

fn opts<'a>(provider: &'a str, model: &'a str) -> RegisterOptions<'a> {
    RegisterOptions {
        provider,
        model,
        command: "run",
        prompt_preview: None,
        session_id: Some("sess-test"),
        session_name: None,
        root: None,
    }
}

#[test]
fn build_env_vars_includes_required_keys() {
    let vars = build_env_vars("proc-abc", &opts("claude", "sonnet"));
    let map: std::collections::HashMap<&str, &str> =
        vars.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

    assert_eq!(map.get("ZAG_PROCESS_ID"), Some(&"proc-abc"));
    assert_eq!(map.get("ZAG_SESSION_ID"), Some(&"sess-test"));
    assert_eq!(map.get("ZAG_PROVIDER"), Some(&"claude"));
    assert_eq!(map.get("ZAG_MODEL"), Some(&"sonnet"));
}

#[test]
fn build_env_vars_omits_session_id_when_absent() {
    let mut o = opts("claude", "sonnet");
    o.session_id = None;
    let vars = build_env_vars("proc-abc", &o);
    assert!(!vars.iter().any(|(k, _)| k == "ZAG_SESSION_ID"));
    // ZAG_PROCESS_ID is always present.
    assert!(
        vars.iter()
            .any(|(k, v)| k == "ZAG_PROCESS_ID" && v == "proc-abc")
    );
}

#[test]
fn build_env_vars_includes_optional_root_and_name() {
    let mut o = opts("claude", "sonnet");
    o.root = Some("/repo");
    o.session_name = Some("book-writer");
    let vars = build_env_vars("proc-abc", &o);
    let map: std::collections::HashMap<&str, &str> =
        vars.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    assert_eq!(map.get("ZAG_ROOT"), Some(&"/repo"));
    assert_eq!(map.get("ZAG_SESSION_NAME"), Some(&"book-writer"));
}

#[test]
fn build_entry_populates_required_fields() {
    let entry = build_entry(
        "proc-abc",
        &opts("claude", "sonnet"),
        Some("parent-proc".to_string()),
        Some("parent-sess".to_string()),
    );
    assert_eq!(entry.id, "proc-abc");
    assert_eq!(entry.pid, std::process::id());
    assert_eq!(entry.session_id.as_deref(), Some("sess-test"));
    assert_eq!(entry.provider, "claude");
    assert_eq!(entry.model, "sonnet");
    assert_eq!(entry.command, "run");
    assert_eq!(entry.status, "running");
    assert!(entry.exit_code.is_none());
    assert!(entry.exited_at.is_none());
    assert_eq!(entry.parent_process_id.as_deref(), Some("parent-proc"));
    assert_eq!(entry.parent_session_id.as_deref(), Some("parent-sess"));
}

#[test]
fn build_entry_propagates_prompt_preview_and_root() {
    let mut o = opts("claude", "sonnet");
    o.prompt_preview = Some("write a story");
    o.root = Some("/repo");
    let entry = build_entry("proc-abc", &o, None, None);
    assert_eq!(entry.prompt.as_deref(), Some("write a story"));
    assert_eq!(entry.root.as_deref(), Some("/repo"));
}

#[test]
fn process_registration_on_spawn_hook_updates_in_memory_store() {
    // Smoke-test that the hook closure does what we documented: looks up
    // its proc_id in a ProcessStore and replaces the pid. We bypass the
    // disk-backed `register` path and operate on an in-memory store
    // directly to avoid touching the user's real `~/.zag/processes.json`.
    let mut store = ProcessStore::default();
    store.add(build_entry(
        "proc-hook",
        &opts("claude", "sonnet"),
        None,
        None,
    ));
    assert_eq!(store.find("proc-hook").unwrap().pid, std::process::id());
    store.update_pid("proc-hook", 4242);
    assert_eq!(store.find("proc-hook").unwrap().pid, 4242);
}
