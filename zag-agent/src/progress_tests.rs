use super::*;

#[test]
fn test_silent_progress_all_methods_callable() {
    let p = SilentProgress;
    p.on_status("status");
    p.on_success("success");
    p.on_warning("warning");
    p.on_error("error");
    p.on_spinner_start("starting");
    p.on_spinner_finish();
    p.on_debug("debug");
}

#[test]
fn test_silent_progress_implements_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<SilentProgress>();
}
