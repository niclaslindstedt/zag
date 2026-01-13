use anyhow::Result;
use std::path::PathBuf;

fn get_pid_file_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".agent").join("session.pid")
}

pub fn write_pid() -> Result<()> {
    let pid_file = get_pid_file_path();
    if let Some(parent) = pid_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&pid_file, std::process::id().to_string())?;
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
