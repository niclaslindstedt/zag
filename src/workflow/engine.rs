use anyhow::{Result, bail};

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
    pub async fn run(&self, workflow_name: &str, agent_override: Option<&str>) -> Result<()> {
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
        if let Some(agent) = agent_override {
            println!("Agent override: {}", agent);
        }
        println!("Run ID: {}", run_ctx.manifest.run_id);
        println!("State directory: {}", run_ctx.state_dir().display());
        println!();

        let result = self
            .execute_workflow(&workflow, &mut run_ctx, agent_override)
            .await;
        let _ = pid::remove_workflow_context();
        result
    }

    /// Resume a paused or failed workflow.
    pub async fn resume(
        &self,
        workflow_name: &str,
        run_id: Option<&str>,
        agent_override: Option<&str>,
    ) -> Result<()> {
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
        if let Some(agent) = agent_override {
            println!("Agent override: {}", agent);
        }
        println!("Run ID: {}", run_id);
        println!("State directory: {}", run_ctx.state_dir().display());
        println!("Previous status: {:?}", run_ctx.manifest.status);
        println!();

        let result = self
            .execute_workflow(&workflow, &mut run_ctx, agent_override)
            .await;
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

    /// Checkpoint the current workflow phase/iteration.
    /// For iteration phases: marks the current iteration as complete so resume will skip it.
    /// For non-iteration phases: just signals completion.
    /// Also signals that the agent has completed its work (allows clean exit).
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

        let phase = run_ctx
            .manifest
            .current_phase
            .clone()
            .unwrap_or_else(|| "?".to_string());
        let iter = run_ctx.manifest.current_iteration;

        // For iteration phases, mark the iteration as complete
        if let Some(iteration) = iter {
            run_ctx.checkpoint_iteration()?;
            println!("Checkpointed: phase={}, iteration={}", phase, iteration);
        } else {
            println!("Checkpointed: phase={}", phase);
        }

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
        agent_override: Option<&str>,
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
        let total_phases = top_level_phases.len();
        for (index, phase) in top_level_phases.iter().enumerate() {
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

            // Check if this is the last remaining phase to execute
            // (i.e., all phases after this one are already completed)
            let is_last_to_execute = top_level_phases
                .iter()
                .skip(index + 1)
                .all(|p| {
                    run_ctx
                        .manifest
                        .phases
                        .get(&p.id)
                        .map(|s| s.status == RunStatus::Completed)
                        .unwrap_or(false)
                });

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
            let mut executor = PhaseExecutor::new(
                workflow,
                &workflow.defaults,
                run_ctx,
                agent_override,
                self.root.as_deref(),
            )?;
            executor.set_is_last_phase(is_last_to_execute);
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
        // Use the comprehensive validation from validate.rs
        let errors = crate::workflow::validate::validate_workflow(workflow);
        
        if !errors.is_empty() {
            bail!("Workflow validation failed:\n  {}", errors.join("\n  "));
        }

        Ok(())
    }
}
