use anyhow::{bail, Result};
use std::collections::HashSet;

use crate::pid::{self, WorkflowContext};

use super::loader::WorkflowLoader;
use super::phase::PhaseExecutor;
use super::state::{RunContext, StateManager};
use super::types::{RunStatus, Workflow};

/// Main workflow engine that orchestrates workflow execution.
///
/// The engine:
/// 1. Loads workflow definitions (embedded or from config)
/// 2. Creates/resumes run contexts with state directories
/// 3. Executes phases in dependency order
/// 4. Handles iteration and nested phases
pub struct WorkflowEngine {
    loader: WorkflowLoader,
    state_manager: StateManager,
    root: Option<String>,
}

impl WorkflowEngine {
    /// Create a new workflow engine.
    ///
    /// If `root` is provided, state directories are created under `<root>/.agent/state/`.
    pub fn new(root: Option<&str>) -> Self {
        Self {
            loader: WorkflowLoader::new(),
            state_manager: StateManager::new(root),
            root: root.map(|s| s.to_string()),
        }
    }

    /// Run a workflow from the beginning.
    pub async fn run(&self, workflow_name: &str) -> Result<()> {
        let workflow = self.loader.load(workflow_name)?;
        let mut run_ctx = self.state_manager.create_run(workflow_name)?;

        // Write workflow context for checkpoint command
        pid::write_workflow_context(&WorkflowContext {
            workflow: workflow_name.to_string(),
            run_id: run_ctx.manifest.run_id.clone(),
            root: self.root.clone(),
        })?;

        println!("Starting workflow: {} v{}", workflow.name, workflow.version);
        if let Some(ref desc) = workflow.description {
            println!("Description: {}", desc);
        }
        println!("Run ID: {}", run_ctx.manifest.run_id);
        println!("State directory: {}", run_ctx.state_dir().display());
        println!();

        let result = self.execute_workflow(&workflow, &mut run_ctx).await;
        let _ = pid::remove_workflow_context();
        result
    }

    /// Resume a paused or failed workflow.
    pub async fn resume(&self, workflow_name: &str, run_id: Option<&str>) -> Result<()> {
        let workflow = self.loader.load(workflow_name)?;

        let run_id = match run_id {
            Some(id) => id.to_string(),
            None => self
                .state_manager
                .find_latest_run(workflow_name)?
                .ok_or_else(|| {
                    anyhow::anyhow!("No previous run found for workflow: {}", workflow_name)
                })?,
        };

        let mut run_ctx = self.state_manager.resume_run(workflow_name, &run_id)?;

        // Write workflow context for checkpoint command
        pid::write_workflow_context(&WorkflowContext {
            workflow: workflow_name.to_string(),
            run_id: run_id.clone(),
            root: self.root.clone(),
        })?;

        println!("Resuming workflow: {} v{}", workflow.name, workflow.version);
        println!("Run ID: {}", run_id);
        println!("State directory: {}", run_ctx.state_dir().display());
        println!("Previous status: {:?}", run_ctx.manifest.status);
        println!();

        let result = self.execute_workflow(&workflow, &mut run_ctx).await;
        let _ = pid::remove_workflow_context();
        result
    }

    /// List available workflows.
    pub fn list_workflows(&self) -> Result<Vec<String>> {
        self.loader.list_available()
    }

    /// List runs for a workflow.
    pub fn list_runs(&self, workflow_name: &str) -> Result<Vec<String>> {
        self.state_manager.list_runs(workflow_name)
    }

    /// Checkpoint the current iteration of a running workflow.
    /// This marks the current iteration as complete so resume will skip it.
    /// If workflow_name is None, tries to auto-detect from active workflow context.
    pub fn checkpoint(workflow_name: Option<&str>, run_id: Option<&str>) -> Result<()> {
        // Try to get context from active workflow, or use provided args
        let (workflow, run, root) = match pid::read_workflow_context()? {
            Some(ctx) => (
                workflow_name.map(|s| s.to_string()).unwrap_or(ctx.workflow),
                run_id.map(|s| s.to_string()).unwrap_or(ctx.run_id),
                ctx.root,
            ),
            None => {
                let wf = workflow_name
                    .ok_or_else(|| anyhow::anyhow!("No active workflow. Provide workflow name."))?;
                let state_mgr = StateManager::new(None);
                let rid = run_id.map(|s| s.to_string()).unwrap_or_else(|| {
                    state_mgr
                        .find_latest_run(wf)
                        .ok()
                        .flatten()
                        .unwrap_or_default()
                });
                (wf.to_string(), rid, None)
            }
        };

        if run.is_empty() {
            bail!("No run ID found for workflow: {}", workflow);
        }

        let state_mgr = StateManager::new(root.as_deref());
        let mut run_ctx = state_mgr.resume_run(&workflow, &run)?;
        run_ctx.checkpoint_iteration()?;

        let phase = run_ctx.manifest.current_phase.as_deref().unwrap_or("?");
        let iter = run_ctx.manifest.current_iteration.unwrap_or(0);
        println!("Checkpointed: phase={}, iteration={}", phase, iter);

        Ok(())
    }

    /// Get workflow info.
    #[allow(dead_code)]
    pub fn get_workflow_info(&self, name: &str) -> Result<Workflow> {
        self.loader.load(name)
    }

    async fn execute_workflow(
        &self,
        workflow: &Workflow,
        run_ctx: &mut RunContext,
    ) -> Result<()> {
        // Validate workflow structure
        self.validate_workflow(workflow)?;

        // Get top-level phases (phases without a parent)
        let top_level_phases: Vec<_> = workflow
            .phases
            .iter()
            .filter(|p| p.parent.is_none())
            .collect();

        // Execute top-level phases in order, respecting dependencies
        for phase in top_level_phases {
            // Check if phase already completed (for resume)
            let is_completed = run_ctx
                .manifest
                .phases
                .get(&phase.id)
                .map(|s| s.status == RunStatus::Completed)
                .unwrap_or(false);

            if is_completed {
                println!("Skipping completed phase: {}", phase.name);
                continue;
            }

            // Check dependencies are satisfied
            for dep in &phase.depends_on {
                let dep_completed = run_ctx
                    .manifest
                    .phases
                    .get(dep)
                    .map(|s| s.status == RunStatus::Completed)
                    .unwrap_or(false);

                if !dep_completed {
                    bail!(
                        "Phase '{}' depends on '{}' which is not completed",
                        phase.id,
                        dep
                    );
                }
            }

            // Create executor for this phase and execute
            let mut executor = PhaseExecutor::new(workflow, &workflow.defaults, run_ctx);
            if let Err(e) = executor.execute_phase(phase).await {
                run_ctx.fail(&e.to_string())?;
                return Err(e);
            }
        }

        run_ctx.complete()?;
        println!("\n=== Workflow completed successfully ===");
        println!("State directory: {}", run_ctx.state_dir().display());
        Ok(())
    }

    fn validate_workflow(&self, workflow: &Workflow) -> Result<()> {
        let phase_ids: HashSet<_> = workflow.phases.iter().map(|p| p.id.as_str()).collect();

        for phase in &workflow.phases {
            // Validate dependencies exist
            for dep in &phase.depends_on {
                if !phase_ids.contains(dep.as_str()) {
                    bail!(
                        "Phase '{}' depends on unknown phase '{}'",
                        phase.id,
                        dep
                    );
                }
            }

            // Validate parent exists
            if let Some(ref parent) = phase.parent {
                if !phase_ids.contains(parent.as_str()) {
                    bail!(
                        "Phase '{}' has unknown parent '{}'",
                        phase.id,
                        parent
                    );
                }
            }

            // Validate nested phases exist
            for nested in &phase.nested_phases {
                if !phase_ids.contains(nested.as_str()) {
                    bail!(
                        "Phase '{}' references unknown nested phase '{}'",
                        phase.id,
                        nested
                    );
                }
            }
        }

        // Check for circular dependencies (simple check)
        self.check_circular_dependencies(workflow)?;

        Ok(())
    }

    fn check_circular_dependencies(&self, workflow: &Workflow) -> Result<()> {
        let dep_map = PhaseExecutor::build_dependency_map(workflow);

        for phase in &workflow.phases {
            let mut visited = HashSet::new();
            let mut stack = vec![phase.id.clone()];

            while let Some(current) = stack.pop() {
                if visited.contains(&current) {
                    bail!(
                        "Circular dependency detected involving phase '{}'",
                        current
                    );
                }
                visited.insert(current.clone());

                if let Some(deps) = dep_map.get(&current) {
                    for dep in deps {
                        if dep == &phase.id {
                            bail!(
                                "Circular dependency: '{}' -> ... -> '{}'",
                                phase.id,
                                dep
                            );
                        }
                        stack.push(dep.clone());
                    }
                }
            }
        }

        Ok(())
    }
}
