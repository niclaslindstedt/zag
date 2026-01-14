use anyhow::{Context, Result};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use crate::interrupt;
use crate::session::{AgentSession, resolve_model_for_agent};

use super::definitions;
use super::memory::{self, MemoryManager};
use super::state::RunContext;
use super::template::TemplateEngine;
use super::types::{ExecutionMode, Phase, PhaseStatus, Workflow, WorkflowDefaults};
use super::variables::VariableResolver;

/// Executes workflow phases, handling iteration and nested phases.
pub struct PhaseExecutor<'a> {
    workflow: &'a Workflow,
    defaults: &'a WorkflowDefaults,
    run_ctx: &'a mut RunContext,
    /// Template engine with accumulated context (epic, ticket, etc.)
    context_template: TemplateEngine,
    /// Optional agent override from CLI (takes precedence over workflow/phase settings)
    agent_override: Option<String>,
    /// Memory manager for loading workflow memories
    memory_manager: MemoryManager,
    /// Whether memory injection is enabled for this workflow
    memory_enabled: bool,
}

impl<'a> PhaseExecutor<'a> {
    pub fn new(
        workflow: &'a Workflow,
        defaults: &'a WorkflowDefaults,
        run_ctx: &'a mut RunContext,
        agent_override: Option<&str>,
        project_root: Option<&str>,
    ) -> Result<Self> {
        let mut context_template = TemplateEngine::new();
        context_template.set_state_dir(&run_ctx.state_dir_str());

        // Resolve workflow variables
        VariableResolver::resolve_all(&workflow.variables, &mut context_template)?;

        // Initialize memory manager
        let memory_manager = MemoryManager::new(project_root, &workflow.name);
        let memory_enabled = defaults.memory;

        Ok(Self {
            workflow,
            defaults,
            run_ctx,
            context_template,
            agent_override: agent_override.map(|s| s.to_string()),
            memory_manager,
            memory_enabled,
        })
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
                    self.run_ctx.update_phase_status(
                        &phase.id,
                        PhaseStatus::failed(started_at, e.to_string()),
                    )?;
                }
            }

            result
        })
    }

    /// Execute a phase once.
    async fn execute_once(&mut self, phase: &Phase) -> Result<()> {
        let session = self.create_session(phase)?;
        self.run_session(session).await
    }

    /// Run a session, detecting if it was interrupted.
    async fn run_session(&self, session: AgentSession) -> Result<()> {
        let result = session.run().await;

        // Check if interrupted via Ctrl+C
        if interrupt::was_interrupted() {
            anyhow::bail!("Session was interrupted");
        }

        result
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

        let items: Vec<serde_json::Value> =
            serde_json::from_str(&items_content).with_context(|| {
                format!(
                    "Failed to parse iteration file as JSON array: {}",
                    items_path
                )
            })?;

        if items.is_empty() && phase.execution.skip_if_empty {
            println!("  Skipping iteration: empty array");
            return Ok(());
        }

        let total = items.len();
        println!("  Iterating over {} items", total);

        for (index, item) in items.iter().enumerate() {
            let iteration_num = index + 1;

            // Skip completed iterations (checkpoint resume)
            if self
                .run_ctx
                .is_iteration_completed(&phase.id, iteration_num)
            {
                let default_id = format!("{}", iteration_num);
                let item_id = item
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&default_id);
                println!(
                    "\n--- Skipping completed iteration {}/{}: {} ---",
                    iteration_num, total, item_id
                );
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
            println!(
                "\n--- Iteration {}/{}: {} ---",
                iteration_num, total, item_id
            );

            // Execute the phase itself (if it has a prompt and no nested phases)
            if !phase.prompt.is_empty() && phase.nested_phases.is_empty() {
                let session = self.create_session(phase)?;
                self.run_session(session).await?;
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

    /// File injection marker documentation (always injected into system prompt)
    const FILE_INJECTION_INFO: &'static str = "## File Injection Markers\n\nWhen file contents are injected into prompts via workflow variables (type: \"file\"), they are wrapped with special delimiters:\n\n```\n///!agent:injected_file_start:<path>\n<file contents>\n///!agent:injected_file_end:<path>\n```\n\nThese markers help you identify which content came from external files and distinguish between multiple injected files. You can reference files by their path shown in the markers.";

    /// Create an AgentSession for a phase.
    fn create_session(&self, phase: &Phase) -> Result<AgentSession> {
        // Agent priority: CLI override > phase setting > workflow default
        let agent = self
            .agent_override
            .clone()
            .or_else(|| phase.agent.clone())
            .unwrap_or_else(|| self.defaults.agent.clone());

        // Model priority: phase setting > workflow default
        // Resolve size aliases (small/medium/large) to actual model names
        let model = phase
            .model
            .as_ref()
            .or(self.defaults.model.as_ref())
            .map(|m| resolve_model_for_agent(&agent, m));

        let interactive = phase.interactive.unwrap_or(self.defaults.interactive);

        let skip_permissions = phase
            .skip_permissions
            .unwrap_or(self.defaults.skip_permissions);

        // Expand templates in prompts
        let mut system_prompt = phase
            .system_prompt
            .as_ref()
            .map(|s| self.context_template.expand(s));

        // Prepend workflow definitions to system prompt
        if let Some(defs) =
            definitions::format_definitions(&self.workflow.definitions, &self.context_template)
        {
            system_prompt = Some(match system_prompt {
                Some(sp) => format!("{}\n\n{}", defs, sp),
                None => defs,
            });
        }

        // Inject file injection documentation (always shown)
        system_prompt = Some(match system_prompt {
            Some(sp) => format!("{}\n\n{}", sp, Self::FILE_INJECTION_INFO),
            None => Self::FILE_INJECTION_INFO.to_string(),
        });

        // Inject workflow memories (after phase system_prompt, before completion instructions)
        if self.memory_enabled {
            if let Ok(entries) = self.memory_manager.load() {
                if let Some(memories) = memory::format_memories(&entries) {
                    system_prompt = Some(match system_prompt {
                        Some(sp) => format!("{}\n\n{}", sp, memories),
                        None => memories,
                    });
                }
            }
        }

        let prompt = self.context_template.expand(&phase.prompt);

        Ok(AgentSession::new(
            agent,
            Some(prompt),
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
