//! Workflow management (create, modify, delete).
//!
//! Provides commands for managing user-defined workflows:
//! - Create: Launch an AI agent to help design and write workflow JSON
//! - Modify: Launch an AI agent to help modify existing workflows
//! - Delete: Remove user-defined workflows from ~/.agent/workflows/

use anyhow::{bail, Result};
use std::path::PathBuf;

use crate::session::{run_sessions, AgentSession};

/// System prompt for workflow creation.
const CREATE_SYSTEM_PROMPT: &str = include_str!("../../prompts/workflow-create-system.md");

/// System prompt for workflow modification.
const MODIFY_SYSTEM_PROMPT: &str = include_str!("../../prompts/workflow-modify-system.md");

/// User prompt template for workflow creation.
const CREATE_USER_PROMPT_TEMPLATE: &str = r#"Help me create a new workflow named "{{name}}".

Your task:
1. Ask me what the workflow should accomplish and what phases it needs
2. Design the phases based on my requirements, using the workflow schema from your system prompt
3. Write the workflow JSON to ~/.agent/workflows/{{name}}.json
4. Explain how to test the workflow

Start by asking me about the workflow's purpose and what it should do."#;

/// User prompt template for workflow modification.
const MODIFY_USER_PROMPT_TEMPLATE: &str = r#"Help me modify the workflow "{{name}}".

The workflow file is located at: {{path}}

Your task:
1. Read the existing workflow file to understand its current structure
2. Ask me what I want to change or what isn't working as expected
3. Make the requested modifications using the workflow schema from your system prompt
4. Explain the changes you made

Start by reading the workflow file, then ask me what I'd like to modify."#;

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
    let user_prompt = CREATE_USER_PROMPT_TEMPLATE.replace("{{name}}", name);

    let session = AgentSession::new(
        agent_name,
        user_prompt,
        Some(CREATE_SYSTEM_PROMPT.to_string()),
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

/// Embedded workflows (must match loader.rs)
const EMBEDDED_WORKFLOWS: &[(&str, &str)] = &[
    ("software", include_str!("../../workflows/software.json")),
];

/// Modify an existing workflow with AI assistance.
///
/// Launches an interactive agent session to help modify a workflow.
/// For embedded workflows, creates a copy in the user directory first.
///
/// # Arguments
///
/// * `name` - Name of the workflow to modify
/// * `agent_name` - Which agent to use ("claude", "codex", "gemini", "copilot")
pub async fn modify_workflow(name: &str, agent_name: &str) -> Result<()> {
    let user_path = get_workflow_path(name);

    // Check if workflow exists (user or embedded)
    let is_embedded = EMBEDDED_WORKFLOWS.iter().any(|(n, _)| *n == name);

    if !user_path.exists() && !is_embedded {
        bail!(
            "Workflow '{}' not found. Use --list to see available workflows.",
            name
        );
    }

    // For embedded workflows without a user override, copy to user directory first
    if !user_path.exists() && is_embedded {
        if let Some((_, content)) = EMBEDDED_WORKFLOWS.iter().find(|(n, _)| *n == name) {
            // Create directory if it doesn't exist
            if let Some(parent) = user_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&user_path, content)?;
            println!(
                "Copied embedded workflow '{}' to {} for modification",
                name,
                user_path.display()
            );
        }
    }

    let user_prompt = MODIFY_USER_PROMPT_TEMPLATE
        .replace("{{name}}", name)
        .replace("{{path}}", &user_path.display().to_string());

    let session = AgentSession::new(
        agent_name,
        user_prompt,
        Some(MODIFY_SYSTEM_PROMPT.to_string()),
        None,  // default model
        None,  // current directory
        false, // require permissions
        true,  // interactive
    );

    run_sessions(vec![session]).await
}
