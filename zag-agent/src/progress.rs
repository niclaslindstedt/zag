/// Progress and status reporting trait for agent operations.
///
/// Library consumers implement this to receive status updates during
/// agent execution. The CLI binary implements this with terminal spinners
/// and colored output; programmatic users can use `SilentProgress` or
/// implement custom reporting.
pub trait ProgressHandler: Send + Sync {
    /// A status message about an ongoing operation.
    fn on_status(&self, _message: &str) {}

    /// An operation completed successfully.
    fn on_success(&self, _message: &str) {}

    /// A non-fatal warning.
    fn on_warning(&self, _message: &str) {}

    /// An error occurred.
    fn on_error(&self, _message: &str) {}

    /// A long-running operation started (e.g., spinner).
    fn on_spinner_start(&self, _message: &str) {}

    /// The current spinner/long-running operation finished.
    fn on_spinner_finish(&self) {}

    /// A debug-level message.
    fn on_debug(&self, _message: &str) {}
}

/// No-op progress handler for library users who don't need status output.
pub struct SilentProgress;

impl ProgressHandler for SilentProgress {}

#[cfg(test)]
#[path = "progress_tests.rs"]
mod tests;
