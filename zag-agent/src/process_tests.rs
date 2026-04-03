use super::*;

#[test]
fn test_check_exit_status_success() {
    // ExitStatus for success (code 0)
    let output = std::process::Command::new("true").output().unwrap();
    assert!(check_exit_status(output.status, "", "Test").is_ok());
    assert!(check_exit_status(output.status, "some stderr", "Test").is_ok());
}

#[test]
fn test_check_exit_status_failure_with_stderr() {
    let output = std::process::Command::new("false").output().unwrap();
    let result = check_exit_status(output.status, "error message", "Agent");
    assert!(result.is_err());
    // When stderr is non-empty, the error should contain the stderr text
    assert_eq!(result.unwrap_err().to_string(), "error message");
}

#[test]
fn test_check_exit_status_failure_without_stderr() {
    let output = std::process::Command::new("false").output().unwrap();
    let result = check_exit_status(output.status, "", "Claude");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Claude command failed with status"));
}

#[test]
fn test_handle_output_success() {
    let output = std::process::Command::new("echo")
        .arg("hello")
        .output()
        .unwrap();
    assert!(handle_output(&output, "Test").is_ok());
}

#[test]
fn test_handle_output_failure() {
    let output = std::process::Command::new("false").output().unwrap();
    assert!(handle_output(&output, "Test").is_err());
}

#[test]
fn test_handle_output_with_stderr_on_success() {
    // Even with stderr output, success should return Ok
    let output = std::process::Command::new("sh")
        .args(["-c", "echo warning >&2; exit 0"])
        .output()
        .unwrap();
    assert!(handle_output(&output, "Test").is_ok());
}

#[test]
fn test_handle_output_with_stderr_on_failure() {
    let output = std::process::Command::new("sh")
        .args(["-c", "echo 'bad thing happened' >&2; exit 1"])
        .output()
        .unwrap();
    let result = handle_output(&output, "Test");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("bad thing happened")
    );
}

#[test]
fn test_log_stderr_text_empty_does_not_panic() {
    log_stderr_text("");
}

#[test]
fn test_log_stderr_text_nonempty_does_not_panic() {
    log_stderr_text("some error\nanother line");
}

#[tokio::test]
async fn test_run_captured_success() {
    let mut cmd = Command::new("echo");
    cmd.arg("hello world");
    let result = run_captured(&mut cmd, "Test").await.unwrap();
    assert_eq!(result, "hello world");
}

#[tokio::test]
async fn test_run_captured_failure() {
    let mut cmd = Command::new("sh");
    cmd.args(["-c", "echo fail >&2; exit 1"]);
    let result = run_captured(&mut cmd, "Test").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("fail"));
}

#[tokio::test]
async fn test_run_captured_trims_output() {
    let mut cmd = Command::new("echo");
    cmd.arg("  padded  ");
    let result = run_captured(&mut cmd, "Test").await.unwrap();
    assert_eq!(result, "padded");
}

#[tokio::test]
async fn test_run_with_captured_stderr_success() {
    let mut cmd = Command::new("echo");
    cmd.arg("hello");
    cmd.stdin(Stdio::null()).stdout(Stdio::null());
    assert!(run_with_captured_stderr(&mut cmd).await.is_ok());
}

#[tokio::test]
async fn test_run_with_captured_stderr_failure() {
    let mut cmd = Command::new("sh");
    cmd.args(["-c", "exit 1"]);
    cmd.stdin(Stdio::null()).stdout(Stdio::null());
    assert!(run_with_captured_stderr(&mut cmd).await.is_err());
}

#[tokio::test]
async fn test_wait_with_stderr_success() {
    let mut cmd = Command::new("echo");
    cmd.arg("test");
    cmd.stdout(Stdio::null());
    let child = spawn_with_captured_stderr(&mut cmd).await.unwrap();
    assert!(wait_with_stderr(child).await.is_ok());
}

#[tokio::test]
async fn test_wait_with_stderr_failure() {
    let mut cmd = Command::new("sh");
    cmd.args(["-c", "echo err >&2; exit 1"]);
    cmd.stdout(Stdio::null());
    let child = spawn_with_captured_stderr(&mut cmd).await.unwrap();
    let result = wait_with_stderr(child).await;
    assert!(result.is_err());
}
