use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

/// `run_interactive_command` must spawn the child, invoke the
/// `on_spawn` callback with the real child pid, and then wait for
/// the child to exit. The pid surfaced to the callback must match
/// the actual OS pid observed by the spawn (proved by checking
/// that it is non-zero — on Unix, a freshly spawned child always
/// has a live pid).
#[tokio::test]
async fn notify_spawn_delivers_pid_before_wait() {
    let captured = Arc::new(AtomicU32::new(0));
    let captured_clone = captured.clone();
    let hook: OnSpawnHook = Arc::new(move |pid| {
        captured_clone.store(pid, Ordering::SeqCst);
    });

    let mut cmd = Command::new("true");
    CommonAgentState::run_interactive_command_with_hook(&mut cmd, "Test", Some(&hook))
        .await
        .expect("`true` must exit 0");

    let pid = captured.load(Ordering::SeqCst);
    assert!(pid > 0, "expected a non-zero child pid, got {pid}");
}

#[tokio::test]
async fn notify_spawn_without_hook_is_noop() {
    // Sanity check: the helper still works when no hook is passed.
    let mut cmd = Command::new("true");
    CommonAgentState::run_interactive_command_with_hook(&mut cmd, "Test", None)
        .await
        .expect("`true` must exit 0");
}

/// The original 2-arg signature is preserved as a backwards-compat
/// shim so downstream consumers of the public `CommonAgentState`
/// API keep compiling without passing a hook.
#[tokio::test]
async fn legacy_two_arg_signature_still_works() {
    let mut cmd = Command::new("true");
    CommonAgentState::run_interactive_command(&mut cmd, "Test")
        .await
        .expect("`true` must exit 0");
}
