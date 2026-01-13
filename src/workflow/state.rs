use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;

use super::types::{PhaseStatus, RunManifest, RunStatus};

/// Manages workflow state directories and run manifests.
///
/// State directory structure:
/// ```text
/// .agent/state/
/// └── <workflow_name>/
///     └── <run_id>/           # Timestamp-based run ID
///         ├── manifest.json   # Run status and progress
///         ├── spec.md         # Phase outputs
///         ├── epics.json
///         └── epics/
///             └── epic-001/
///                 ├── tickets.json
///                 └── tickets/
///                     └── T001/
///                         └── ...
/// ```
pub struct StateManager {
    base_dir: PathBuf,
}

impl StateManager {
    /// Create a new state manager.
    ///
    /// If `root` is provided, state directory is `<root>/.agent/state/`.
    /// Otherwise uses current working directory.
    pub fn new(root: Option<&str>) -> Self {
        let base = root
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        Self {
            base_dir: base.join(".agent").join("state"),
        }
    }

    /// Create a new run for a workflow.
    ///
    /// Returns a `RunContext` with a fresh state directory and manifest.
    pub fn create_run(&self, workflow_name: &str) -> Result<RunContext> {
        let run_id = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let run_dir = self.base_dir.join(workflow_name).join(&run_id);

        std::fs::create_dir_all(&run_dir)
            .with_context(|| format!("Failed to create run directory: {}", run_dir.display()))?;

        let manifest = RunManifest {
            workflow: workflow_name.to_string(),
            run_id: run_id.clone(),
            started_at: Utc::now().to_rfc3339(),
            status: RunStatus::Pending,
            current_phase: None,
            current_epic: None,
            current_ticket: None,
            current_iteration: None,
            total_iterations: None,
            phases: HashMap::new(),
        };

        let ctx = RunContext { run_dir, manifest };
        ctx.save_manifest()?;

        Ok(ctx)
    }

    /// Resume an existing run by its ID.
    pub fn resume_run(&self, workflow_name: &str, run_id: &str) -> Result<RunContext> {
        let run_dir = self.base_dir.join(workflow_name).join(run_id);
        let manifest_path = run_dir.join("manifest.json");

        let content = std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("Failed to read manifest: {}", manifest_path.display()))?;
        let manifest: RunManifest = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse manifest: {}", manifest_path.display()))?;

        Ok(RunContext { run_dir, manifest })
    }

    /// Find the most recent run for a workflow.
    pub fn find_latest_run(&self, workflow_name: &str) -> Result<Option<String>> {
        let workflow_dir = self.base_dir.join(workflow_name);
        if !workflow_dir.exists() {
            return Ok(None);
        }

        let mut runs: Vec<String> = std::fs::read_dir(&workflow_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();

        // Sort descending (newest first since format is YYYYMMDD_HHMMSS)
        runs.sort();
        runs.reverse();

        Ok(runs.into_iter().next())
    }

    /// List all runs for a workflow.
    pub fn list_runs(&self, workflow_name: &str) -> Result<Vec<String>> {
        let workflow_dir = self.base_dir.join(workflow_name);
        if !workflow_dir.exists() {
            return Ok(vec![]);
        }

        let mut runs: Vec<String> = std::fs::read_dir(&workflow_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();

        runs.sort();
        runs.reverse();

        Ok(runs)
    }
}

/// Context for a single workflow run, including state directory and manifest.
pub struct RunContext {
    pub run_dir: PathBuf,
    pub manifest: RunManifest,
}

impl RunContext {
    /// Get the state directory path.
    pub fn state_dir(&self) -> &PathBuf {
        &self.run_dir
    }

    /// Get the state directory as a string.
    pub fn state_dir_str(&self) -> String {
        self.run_dir.display().to_string()
    }

    /// Save the manifest to disk.
    pub fn save_manifest(&self) -> Result<()> {
        let path = self.run_dir.join("manifest.json");
        let content = serde_json::to_string_pretty(&self.manifest)
            .context("Failed to serialize manifest")?;
        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write manifest: {}", path.display()))?;
        Ok(())
    }

    /// Update the status of a phase.
    pub fn update_phase_status(&mut self, phase_id: &str, status: PhaseStatus) -> Result<()> {
        self.manifest.phases.insert(phase_id.to_string(), status);
        self.save_manifest()
    }

    /// Set the currently executing phase.
    pub fn set_current_phase(&mut self, phase_id: &str) -> Result<()> {
        self.manifest.current_phase = Some(phase_id.to_string());
        self.manifest.status = RunStatus::InProgress;
        self.save_manifest()
    }

    /// Set the current epic being processed.
    pub fn set_current_epic(&mut self, epic_id: Option<&str>) -> Result<()> {
        self.manifest.current_epic = epic_id.map(|s| s.to_string());
        self.save_manifest()
    }

    /// Set the current ticket being processed.
    pub fn set_current_ticket(&mut self, ticket_id: Option<&str>) -> Result<()> {
        self.manifest.current_ticket = ticket_id.map(|s| s.to_string());
        self.save_manifest()
    }

    /// Set iteration progress.
    pub fn set_iteration(&mut self, current: usize, total: usize) -> Result<()> {
        self.manifest.current_iteration = Some(current);
        self.manifest.total_iterations = Some(total);
        self.save_manifest()
    }

    /// Mark the current iteration as completed (checkpoint).
    pub fn checkpoint_iteration(&mut self) -> Result<()> {
        let phase_id = self
            .manifest
            .current_phase
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No current phase"))?;
        let iteration = self
            .manifest
            .current_iteration
            .ok_or_else(|| anyhow::anyhow!("No current iteration"))?;

        if let Some(status) = self.manifest.phases.get_mut(&phase_id) {
            if !status.completed_iterations.contains(&iteration) {
                status.completed_iterations.push(iteration);
            }
        }
        self.save_manifest()
    }

    /// Check if an iteration is already completed.
    pub fn is_iteration_completed(&self, phase_id: &str, iteration: usize) -> bool {
        self.manifest
            .phases
            .get(phase_id)
            .map(|s| s.completed_iterations.contains(&iteration))
            .unwrap_or(false)
    }

    /// Mark the run as completed.
    pub fn complete(&mut self) -> Result<()> {
        self.manifest.status = RunStatus::Completed;
        self.manifest.current_phase = None;
        self.manifest.current_iteration = None;
        self.manifest.total_iterations = None;
        self.save_manifest()
    }

    /// Mark the run as failed.
    pub fn fail(&mut self, error: &str) -> Result<()> {
        self.manifest.status = RunStatus::Failed;
        if let Some(ref phase_id) = self.manifest.current_phase.clone() {
            if let Some(status) = self.manifest.phases.get_mut(phase_id) {
                status.status = RunStatus::Failed;
                status.error = Some(error.to_string());
            }
        }
        self.save_manifest()
    }

    /// Mark the run as paused (can be resumed later).
    pub fn pause(&mut self) -> Result<()> {
        self.manifest.status = RunStatus::Paused;
        self.save_manifest()
    }

    /// Create a subdirectory within the state directory.
    pub fn create_subdir(&self, path: &str) -> Result<PathBuf> {
        let dir = self.run_dir.join(path);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create directory: {}", dir.display()))?;
        Ok(dir)
    }

    /// Write content to a file in the state directory.
    pub fn write_file(&self, path: &str, content: &str) -> Result<()> {
        let file_path = self.run_dir.join(path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent directory: {}", parent.display()))?;
        }
        std::fs::write(&file_path, content)
            .with_context(|| format!("Failed to write file: {}", file_path.display()))?;
        Ok(())
    }

    /// Read content from a file in the state directory.
    pub fn read_file(&self, path: &str) -> Result<String> {
        let file_path = self.run_dir.join(path);
        std::fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))
    }

    /// Check if a file exists in the state directory.
    pub fn file_exists(&self, path: &str) -> bool {
        self.run_dir.join(path).exists()
    }
}
