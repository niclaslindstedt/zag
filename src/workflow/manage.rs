//! Workflow management (create, delete).
//!
//! Provides commands for managing user-defined workflows:
//! - Create: Launch an AI agent to help design and write workflow JSON
//! - Delete: Remove user-defined workflows from ~/.agent/workflows/

use anyhow::{bail, Result};
use std::path::PathBuf;

use crate::session::{run_sessions, AgentSession};

/// System prompt embedded at compile time.
const SYSTEM_PROMPT: &str = include_str!("../../prompts/workflow-create-system.md");

/// User prompt template for workflow creation.
const USER_PROMPT_TEMPLATE: &str = r#"Create a new workflow named "{{name}}".

Instructions:
1. Ask me what the workflow should accomplish
2. Design the phases based on my requirements
3. Write the workflow JSON to ~/.agent/workflows/{{name}}.json
4. Provide testing instructions for the workflow

Start by asking me about the workflow's purpose."#;

/// Create a new workflow with AI assistance.
///
/// Launches an interactive agent session with embedded prompts that guide
/// the user through workflow creation.
///
/// # Arguments
///
/// * `name` - Name for the new workflow (used in filename and prompts)
/// * `agent_name` - Which agent to use ("claude", "codex", "gemini", "copilot")
pub async fn create_workflow(name: &str, agent_name: &str) -> Result<()> {
    let user_prompt = USER_PROMPT_TEMPLATE.replace("{{name}}", name);

    let session = AgentSession::new(
        agent_name,
        user_prompt,
        Some(SYSTEM_PROMPT.to_string()),
        None,  // default model
        None,  // current directory
        false, // require permissions
        true,  // interactive
    );

    run_sessions(vec![session]).await
}

/// Get the path to a user-defined workflow file.
fn get_workflow_path(name: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home)
        .join(".agent")
        .join("workflows")
        .join(format!("{}.json", name))
}

/// Delete a user-defined workflow.
///
/// Only deletes workflows from the user config directory (~/.agent/workflows/).
/// Embedded workflows cannot be deleted.
///
/// # Arguments
///
/// * `name` - Name of the workflow to delete
pub fn delete_workflow(name: &str) -> Result<()> {
    // Check if it's an embedded workflow
    let embedded = ["software"];
    if embedded.contains(&name) {
        bail!(
            "Cannot delete embedded workflow '{}'. Only user-defined workflows can be deleted.",
            name
        );
    }

    let path = get_workflow_path(name);

    if !path.exists() {
        bail!(
            "Workflow '{}' not found at {}",
            name,
            path.display()
        );
    }

    std::fs::remove_file(&path)?;
    println!("Deleted workflow: {}", name);

    Ok(())
}
