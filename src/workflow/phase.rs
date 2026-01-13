use anyhow::{Context, Result};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use crate::session::AgentSession;

use super::state::RunContext;
use super::template::TemplateEngine;
use super::types::{ExecutionMode, Phase, PhaseStatus, Workflow, WorkflowDefaults};

/// Executes workflow phases, handling iteration and nested phases.
pub struct PhaseExecutor<'a> {
    workflow: &'a Workflow,
    defaults: &'a WorkflowDefaults,
    run_ctx: &'a mut RunContext,
    /// Template engine with accumulated context (epic, ticket, etc.)
    context_template: TemplateEngine,
}

impl<'a> PhaseExecutor<'a> {
    pub fn new(
        workflow: &'a Workflow,
        defaults: &'a WorkflowDefaults,
        run_ctx: &'a mut RunContext,
    ) -> Self {
        let mut context_template = TemplateEngine::new();
        context_template.set_state_dir(&run_ctx.state_dir_str());

        Self {
            workflow,
            defaults,
            run_ctx,
            context_template,
        }
    }

    /// Execute a phase with the current context.
    ///
    /// Returns a boxed future to allow for recursive async calls.
    pub fn execute_phase<'b>(
        &'b mut self,
        phase: &'b Phase,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'b>> {
        Box::pin(async move {
            self.run_ctx.set_current_phase(&phase.id)?;
            // Use in_progress_preserving to keep completed_iterations from previous runs
            let existing = self.run_ctx.manifest.phases.get(&phase.id);
            self.run_ctx
                .update_phase_status(&phase.id, PhaseStatus::in_progress_preserving(existing))?;

            println!("\n=== Phase: {} ===", phase.name);

            let result = match phase.execution.mode {
                ExecutionMode::Once => self.execute_once(phase).await,
                ExecutionMode::Iterate => self.execute_iterate(phase).await,
            };

            match &result {
                Ok(()) => {
                    let started_at = self
                        .run_ctx
                        .manifest
                        .phases
                        .get(&phase.id)
                        .and_then(|s| s.started_at.clone());
                    self.run_ctx
                        .update_phase_status(&phase.id, PhaseStatus::completed(started_at))?;
                }
                Err(e) => {
                    let started_at = self
                        .run_ctx
                        .manifest
                        .phases
                        .get(&phase.id)
                        .and_then(|s| s.started_at.clone());
                    self.run_ctx
                        .update_phase_status(&phase.id, PhaseStatus::failed(started_at, e.to_string()))?;
                }
            }

            result
        })
    }

    /// Execute a phase once.
    async fn execute_once(&mut self, phase: &Phase) -> Result<()> {
        let session = self.create_session(phase)?;
        session.run().await
    }

    /// Execute a phase for each item in an iteration file.
    async fn execute_iterate(&mut self, phase: &Phase) -> Result<()> {
        let iterate_over = phase
            .execution
            .iterate_over
            .as_ref()
            .context("iterate_over is required for iterate mode")?;

        // Expand the iterate_over path with current context
        let items_path = self.context_template.expand(iterate_over);

        // Check if file exists
        if !std::path::Path::new(&items_path).exists() {
            if phase.execution.skip_if_empty {
                println!("  Skipping iteration: file not found ({})", items_path);
                return Ok(());
            }
            anyhow::bail!("Iteration file not found: {}", items_path);
        }

        let items_content = std::fs::read_to_string(&items_path)
            .with_context(|| format!("Failed to read iteration file: {}", items_path))?;

        let items: Vec<serde_json::Value> = serde_json::from_str(&items_content)
            .with_context(|| format!("Failed to parse iteration file as JSON array: {}", items_path))?;

        if items.is_empty() && phase.execution.skip_if_empty {
            println!("  Skipping iteration: empty array");
            return Ok(());
        }

        let total = items.len();
        println!("  Iterating over {} items", total);

        for (index, item) in items.iter().enumerate() {
            let iteration_num = index + 1;

            // Skip completed iterations (checkpoint resume)
            if self.run_ctx.is_iteration_completed(&phase.id, iteration_num) {
                let default_id = format!("{}", iteration_num);
                let item_id = item
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&default_id);
                println!("\n--- Skipping completed iteration {}/{}: {} ---", iteration_num, total, item_id);
                continue;
            }

            self.run_ctx.set_iteration(iteration_num, total)?;

            // Set item variable in context
            self.context_template
                .set_from_json(&phase.execution.item_variable, item);
            self.context_template.set("index", index.to_string());

            // Log iteration progress
            let default_id = format!("{}", iteration_num);
            let item_id = item
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or(&default_id);
            println!("\n--- Iteration {}/{}: {} ---", iteration_num, total, item_id);

            // Execute the phase itself (if it has a prompt and no nested phases)
            if !phase.prompt.is_empty() && phase.nested_phases.is_empty() {
                let session = self.create_session(phase)?;
                session.run().await?;
            }

            // Execute nested phases
            for nested_phase_id in &phase.nested_phases {
                let nested_phase = self
                    .workflow
                    .phases
                    .iter()
                    .find(|p| &p.id == nested_phase_id)
                    .with_context(|| format!("Nested phase not found: {}", nested_phase_id))?
                    .clone();

                // Recursively execute nested phase (using boxed future)
                self.execute_phase(&nested_phase).await?;
            }
        }

        Ok(())
    }

    /// Completion instruction automatically appended to interactive phase system prompts.
    const COMPLETION_INSTRUCTION: &'static str = "\n\n---\nWORKFLOW COMPLETION: When you have completed your task, you MUST run these commands in order:\n1. `agent workflow --checkpoint` - saves progress so workflow can resume from here\n2. `agent kill` - signals completion and continues to the next phase\nDo not forget these steps.";

    /// Create an AgentSession for a phase.
    fn create_session(&self, phase: &Phase) -> Result<AgentSession> {
        let agent = phase
            .agent
            .as_ref()
            .unwrap_or(&self.defaults.agent)
            .clone();

        let model = phase
            .model
            .as_ref()
            .or(self.defaults.model.as_ref())
            .cloned();

        let interactive = phase.interactive.unwrap_or(self.defaults.interactive);

        let skip_permissions = phase
            .skip_permissions
            .unwrap_or(self.defaults.skip_permissions);

        // Expand templates in prompts
        let mut system_prompt = phase
            .system_prompt
            .as_ref()
            .map(|s| self.context_template.expand(s));

        // For interactive sessions, automatically inject completion instruction
        if interactive {
            system_prompt = Some(match system_prompt {
                Some(sp) => format!("{}{}", sp, Self::COMPLETION_INSTRUCTION),
                None => Self::COMPLETION_INSTRUCTION.trim_start().to_string(),
            });
        }

        let prompt = self.context_template.expand(&phase.prompt);

        Ok(AgentSession::new(
            agent,
            prompt,
            system_prompt,
            model,
            None, // root - use current directory
            skip_permissions,
            interactive,
        ))
    }

    /// Get phases that have no parent (top-level phases).
    pub fn get_top_level_phases(workflow: &Workflow) -> Vec<&Phase> {
        workflow
            .phases
            .iter()
            .filter(|p| p.parent.is_none())
            .collect()
    }

    /// Build a map of phase dependencies for validation.
    pub fn build_dependency_map(workflow: &Workflow) -> HashMap<String, Vec<String>> {
        workflow
            .phases
            .iter()
            .map(|p| (p.id.clone(), p.depends_on.clone()))
            .collect()
    }
}
