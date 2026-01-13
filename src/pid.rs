use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn get_agent_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".agent")
}

fn get_pid_file_path() -> PathBuf {
    get_agent_dir().join("session.pid")
}

fn get_workflow_context_path() -> PathBuf {
    get_agent_dir().join("workflow.json")
}

fn get_killed_marker_path() -> PathBuf {
    get_agent_dir().join("session.killed")
}

pub fn write_pid() -> Result<()> {
    write_pid_for(std::process::id())
}

pub fn write_pid_for(pid: u32) -> Result<()> {
    let pid_file = get_pid_file_path();
    if let Some(parent) = pid_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&pid_file, pid.to_string())?;
    Ok(())
}

pub fn read_pid() -> Result<Option<u32>> {
    let pid_file = get_pid_file_path();
    if !pid_file.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&pid_file)?;
    Ok(content.trim().parse().ok())
}

pub fn remove_pid() -> Result<()> {
    let pid_file = get_pid_file_path();
    if pid_file.exists() {
        std::fs::remove_file(&pid_file)?;
    }
    Ok(())
}

/// Mark that the current session was killed (for workflow to detect interruption)
pub fn write_killed_marker() -> Result<()> {
    let marker = get_killed_marker_path();
    if let Some(parent) = marker.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&marker, "")?;
    Ok(())
}

/// Check if the session was killed and clear the marker
pub fn was_killed() -> bool {
    let marker = get_killed_marker_path();
    if marker.exists() {
        let _ = std::fs::remove_file(&marker);
        true
    } else {
        false
    }
}

/// Active workflow context for checkpoint/resume
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowContext {
    pub workflow: String,
    pub run_id: String,
    pub root: Option<String>,
}

pub fn write_workflow_context(ctx: &WorkflowContext) -> Result<()> {
    let path = get_workflow_context_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string(ctx)?;
    std::fs::write(&path, content)?;
    Ok(())
}

pub fn read_workflow_context() -> Result<Option<WorkflowContext>> {
    let path = get_workflow_context_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&content).ok())
}

pub fn remove_workflow_context() -> Result<()> {
    let path = get_workflow_context_path();
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}
