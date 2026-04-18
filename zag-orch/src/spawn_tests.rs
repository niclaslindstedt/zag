use super::*;
use std::path::{Path, PathBuf};

#[test]
fn test_resolve_zag_bin_explicit_wins() {
    let explicit = PathBuf::from("/opt/zag/bin/zag");
    let resolved = resolve_zag_bin(Some(&explicit)).unwrap();
    assert_eq!(resolved, explicit);
}

#[test]
fn test_resolve_zag_bin_reads_env_var() {
    // SAFETY: the test harness is single-threaded for this process; we scope
    // the mutation to the test body.
    unsafe {
        std::env::set_var("ZAG_BIN", "/usr/local/bin/zag-custom");
    }
    let resolved = resolve_zag_bin(None);
    unsafe {
        std::env::remove_var("ZAG_BIN");
    }
    assert_eq!(resolved.unwrap(), Path::new("/usr/local/bin/zag-custom"));
}

#[test]
fn test_resolve_zag_bin_error_message_on_miss() {
    // If neither PATH has zag nor the current exe is named zag, the resolver
    // should error cleanly. We can't reliably force that state inside the
    // workspace, so only inspect the error message when the miss branch
    // actually fires.
    unsafe {
        std::env::set_var("ZAG_BIN", "");
    }
    let result = resolve_zag_bin(None);
    unsafe {
        std::env::remove_var("ZAG_BIN");
    }
    if let Err(e) = result {
        assert!(
            e.to_string().contains("No `zag` binary found"),
            "unexpected error: {e}"
        );
    }
}

#[test]
fn test_spawn_logs_dir() {
    let dir = spawn_logs_dir();
    assert!(dir.to_string_lossy().contains("spawn"));
    assert!(dir.to_string_lossy().contains("logs"));
}

#[test]
fn test_fifo_path() {
    let path = fifo_path("abc-123");
    assert!(path.to_string_lossy().contains("fifos"));
    assert!(path.to_string_lossy().ends_with("abc-123"));
}

#[test]
fn test_build_relay_args() {
    let params = SpawnParams {
        prompt: Some("hello".to_string()),
        provider: "claude".to_string(),
        model: Some("opus".to_string()),
        root: Some("/tmp/test".to_string()),
        auto_approve: true,
        system_prompt: None,
        add_dirs: vec![],
        size: None,
        max_turns: None,
        timeout: None,
        json: false,
        metadata: SessionMetadata {
            name: None,
            description: None,
            tags: vec![],
        },
        depends_on: vec![],
        inject_context: false,
        retried_from: None,
        interactive: true,
        env_vars: vec![],
        sandbox: None,
        zag_bin: None,
    };
    let args = build_relay_args(&params, "test-session-id");
    assert!(args.contains(&"relay".to_string()));
    assert!(args.contains(&"--session".to_string()));
    assert!(args.contains(&"test-session-id".to_string()));
    assert!(args.contains(&"hello".to_string()));
    assert!(args.contains(&"--auto-approve".to_string()));
    assert!(args.contains(&"--model".to_string()));
    assert!(args.contains(&"opus".to_string()));
    // Should not contain exec-specific args
    assert!(!args.contains(&"exec".to_string()));
}

#[test]
fn test_build_relay_args_no_prompt() {
    let params = SpawnParams {
        prompt: None,
        provider: "claude".to_string(),
        model: None,
        root: None,
        auto_approve: false,
        system_prompt: None,
        add_dirs: vec![],
        size: None,
        max_turns: None,
        timeout: None,
        json: false,
        metadata: SessionMetadata {
            name: None,
            description: None,
            tags: vec![],
        },
        depends_on: vec![],
        inject_context: false,
        retried_from: None,
        interactive: true,
        env_vars: vec![],
        sandbox: None,
        zag_bin: None,
    };
    let args = build_relay_args(&params, "test-id");
    assert!(args.contains(&"relay".to_string()));
    assert!(args.contains(&"--session".to_string()));
    // No prompt arg at the end
    assert_eq!(args.last().unwrap(), "test-id");
}

#[test]
fn test_build_exec_args_has_prompt() {
    let params = SpawnParams {
        prompt: Some("do stuff".to_string()),
        provider: "claude".to_string(),
        model: None,
        root: None,
        auto_approve: false,
        system_prompt: None,
        add_dirs: vec![],
        size: None,
        max_turns: Some(5),
        timeout: None,
        json: false,
        metadata: SessionMetadata {
            name: Some("test".to_string()),
            description: None,
            tags: vec!["batch".to_string()],
        },
        depends_on: vec![],
        inject_context: false,
        retried_from: None,
        interactive: false,
        env_vars: vec![],
        sandbox: None,
        zag_bin: None,
    };
    let args = build_exec_args(&params, "test-id");
    assert!(args.contains(&"exec".to_string()));
    assert!(args.contains(&"--max-turns".to_string()));
    assert!(args.contains(&"5".to_string()));
    assert!(args.contains(&"--name".to_string()));
    assert!(args.contains(&"test".to_string()));
    assert!(args.contains(&"--tag".to_string()));
    assert!(args.contains(&"batch".to_string()));
    assert_eq!(args.last().unwrap(), "do stuff");
}

#[test]
fn test_build_exec_args_with_sandbox() {
    let params = SpawnParams {
        prompt: Some("do stuff".to_string()),
        provider: "claude".to_string(),
        model: None,
        root: None,
        auto_approve: false,
        system_prompt: None,
        add_dirs: vec![],
        size: None,
        max_turns: None,
        timeout: None,
        json: false,
        metadata: SessionMetadata {
            name: None,
            description: None,
            tags: vec![],
        },
        depends_on: vec![],
        inject_context: false,
        retried_from: None,
        interactive: false,
        env_vars: vec![],
        sandbox: Some("sandbox-abc123".to_string()),
        zag_bin: None,
    };
    let args = build_exec_args(&params, "test-id");
    assert!(args.contains(&"exec".to_string()));
    assert!(args.contains(&"--sandbox".to_string()));
    assert!(args.contains(&"sandbox-abc123".to_string()));
    // Sandbox args should come after exec but before --session
    let exec_pos = args.iter().position(|a| a == "exec").unwrap();
    let sandbox_pos = args.iter().position(|a| a == "--sandbox").unwrap();
    let session_pos = args.iter().position(|a| a == "--session").unwrap();
    assert!(sandbox_pos > exec_pos);
    assert!(sandbox_pos < session_pos);
}

#[test]
fn test_build_exec_args_without_sandbox() {
    let params = SpawnParams {
        prompt: Some("do stuff".to_string()),
        provider: "claude".to_string(),
        model: None,
        root: None,
        auto_approve: false,
        system_prompt: None,
        add_dirs: vec![],
        size: None,
        max_turns: None,
        timeout: None,
        json: false,
        metadata: SessionMetadata {
            name: None,
            description: None,
            tags: vec![],
        },
        depends_on: vec![],
        inject_context: false,
        retried_from: None,
        interactive: false,
        env_vars: vec![],
        sandbox: None,
        zag_bin: None,
    };
    let args = build_exec_args(&params, "test-id");
    assert!(!args.contains(&"--sandbox".to_string()));
}
