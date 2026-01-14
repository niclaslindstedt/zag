//! Workflow memory system for persistent learnings across phases.
//!
//! Memories are stored in `.agent/workflows/<workflow_name>/memory.jsonl` (project-level)
//! and injected into system prompts to help agents learn from previous interactions.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

/// A single memory entry with timestamp, content, and optional metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique index for this memory (1-based, assigned sequentially)
    pub id: usize,
    /// ISO 8601 timestamp when the memory was added
    pub timestamp: DateTime<Utc>,
    /// The memory content
    pub content: String,
    /// Optional category for organizing memories
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Optional phase context (which phase was active when memory was added)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
}

impl MemoryEntry {
    /// Create a new memory entry with the current timestamp
    pub fn new(
        id: usize,
        content: String,
        category: Option<String>,
        phase: Option<String>,
    ) -> Self {
        Self {
            id,
            timestamp: Utc::now(),
            content,
            category,
            phase,
        }
    }
}

/// Manages memory storage and retrieval for a workflow
pub struct MemoryManager {
    /// Path to the memory file
    memory_file: PathBuf,
}

impl MemoryManager {
    /// Create a manager for a workflow's memory file.
    ///
    /// The memory file is located at `.agent/workflows/<workflow_name>/memory.jsonl`
    /// relative to the project root.
    pub fn new(root: Option<&str>, workflow_name: &str) -> Self {
        let base = root
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        Self {
            memory_file: base
                .join(".agent")
                .join("workflows")
                .join(workflow_name)
                .join("memory.jsonl"),
        }
    }

    /// Get the path to the memory file
    pub fn memory_file_path(&self) -> &PathBuf {
        &self.memory_file
    }

    /// Add a new memory entry
    pub fn add(
        &self,
        content: String,
        category: Option<String>,
        phase: Option<String>,
    ) -> Result<usize> {
        // Load existing to get next ID
        let existing = self.load().unwrap_or_default();
        let next_id = existing.iter().map(|e| e.id).max().unwrap_or(0) + 1;

        // Ensure parent directory exists
        if let Some(parent) = self.memory_file.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create memory directory: {}", parent.display())
            })?;
        }

        let entry = MemoryEntry::new(next_id, content, category, phase);
        let json = serde_json::to_string(&entry).context("Failed to serialize memory entry")?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.memory_file)
            .with_context(|| {
                format!("Failed to open memory file: {}", self.memory_file.display())
            })?;

        writeln!(file, "{json}").context("Failed to write memory entry")?;

        Ok(next_id)
    }

    /// Load all memory entries from the file
    pub fn load(&self) -> Result<Vec<MemoryEntry>> {
        if !self.memory_file.exists() {
            return Ok(Vec::new());
        }

        let file = fs::File::open(&self.memory_file).with_context(|| {
            format!("Failed to open memory file: {}", self.memory_file.display())
        })?;

        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for (line_num, line) in reader.lines().enumerate() {
            let line = line.with_context(|| format!("Failed to read line {}", line_num + 1))?;
            if line.trim().is_empty() {
                continue;
            }

            let entry: MemoryEntry = serde_json::from_str(&line).with_context(|| {
                format!("Failed to parse memory entry on line {}", line_num + 1)
            })?;
            entries.push(entry);
        }

        Ok(entries)
    }

    /// Remove a memory by its ID
    pub fn remove(&self, id: usize) -> Result<bool> {
        let entries = self.load()?;
        let original_len = entries.len();
        let filtered: Vec<_> = entries.into_iter().filter(|e| e.id != id).collect();

        if filtered.len() == original_len {
            return Ok(false); // ID not found
        }

        // Rewrite the file with remaining entries
        self.rewrite(&filtered)?;
        Ok(true)
    }

    /// Clear all memories (delete the file)
    pub fn clear(&self) -> Result<()> {
        if self.memory_file.exists() {
            fs::remove_file(&self.memory_file).with_context(|| {
                format!(
                    "Failed to delete memory file: {}",
                    self.memory_file.display()
                )
            })?;
        }
        Ok(())
    }

    /// Search memories by content or category
    pub fn search(&self, query: &str) -> Result<Vec<MemoryEntry>> {
        let entries = self.load()?;
        let query_lower = query.to_lowercase();

        Ok(entries
            .into_iter()
            .filter(|e| {
                e.content.to_lowercase().contains(&query_lower)
                    || e.category
                        .as_ref()
                        .map(|c| c.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
            })
            .collect())
    }

    /// List memories with optional category filter (human-readable format)
    pub fn list(&self, category: Option<&str>) -> Result<Vec<String>> {
        let entries = self.load()?;

        let filtered: Vec<_> = if let Some(cat) = category {
            let cat_lower = cat.to_lowercase();
            entries
                .into_iter()
                .filter(|e| {
                    e.category
                        .as_ref()
                        .map(|c| c.to_lowercase() == cat_lower)
                        .unwrap_or(false)
                })
                .collect()
        } else {
            entries
        };

        Ok(filtered
            .iter()
            .map(|e| {
                let mut parts = vec![format!("[{}]", e.id)];

                if let Some(ref cat) = e.category {
                    parts.push(format!("[{}]", cat));
                }

                parts.push(e.content.clone());

                if let Some(ref phase) = e.phase {
                    parts.push(format!("(phase: {})", phase));
                }

                parts.join(" ")
            })
            .collect())
    }

    /// Get all unique categories
    pub fn categories(&self) -> Result<Vec<String>> {
        let entries = self.load()?;
        let mut cats: Vec<_> = entries.iter().filter_map(|e| e.category.clone()).collect();
        cats.sort();
        cats.dedup();
        Ok(cats)
    }

    /// Rewrite the memory file with new entries
    fn rewrite(&self, entries: &[MemoryEntry]) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.memory_file.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = fs::File::create(&self.memory_file).with_context(|| {
            format!(
                "Failed to create memory file: {}",
                self.memory_file.display()
            )
        })?;

        for entry in entries {
            let json = serde_json::to_string(entry)?;
            writeln!(file, "{json}")?;
        }

        Ok(())
    }
}

/// Format memories for injection into system prompt.
///
/// Returns `None` if there are no memories.
/// Format is markdown with a "## Workflow Memories" header.
pub fn format_memories(entries: &[MemoryEntry]) -> Option<String> {
    if entries.is_empty() {
        return None;
    }

    let mut output = String::from("## Workflow Memories\n\n");
    output.push_str("The following are learnings from previous phases in this workflow:\n\n");

    // Group by category
    let mut uncategorized: Vec<&MemoryEntry> = Vec::new();
    let mut by_category: std::collections::BTreeMap<&str, Vec<&MemoryEntry>> =
        std::collections::BTreeMap::new();

    for entry in entries {
        if let Some(ref cat) = entry.category {
            by_category.entry(cat.as_str()).or_default().push(entry);
        } else {
            uncategorized.push(entry);
        }
    }

    // Output uncategorized first
    for entry in &uncategorized {
        let phase_context = entry
            .phase
            .as_ref()
            .map(|p| format!(" (from phase: {})", p))
            .unwrap_or_default();
        output.push_str(&format!("- {}{}\n", entry.content, phase_context));
    }

    // Output categorized
    for (category, entries) in &by_category {
        output.push_str(&format!("\n### {}\n\n", to_title_case(category)));
        for entry in entries {
            let phase_context = entry
                .phase
                .as_ref()
                .map(|p| format!(" (from phase: {})", p))
                .unwrap_or_default();
            output.push_str(&format!("- {}{}\n", entry.content, phase_context));
        }
    }

    Some(output.trim_end().to_string())
}

/// Convert snake_case or kebab-case to Title Case
fn to_title_case(s: &str) -> String {
    s.split(|c| c == '_' || c == '-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_memories_empty() {
        assert!(format_memories(&[]).is_none());
    }

    #[test]
    fn test_format_memories_basic() {
        let entries = vec![
            MemoryEntry::new(1, "First learning".to_string(), None, None),
            MemoryEntry::new(
                2,
                "Second learning".to_string(),
                None,
                Some("spec".to_string()),
            ),
        ];

        let result = format_memories(&entries).unwrap();
        assert!(result.contains("## Workflow Memories"));
        assert!(result.contains("- First learning"));
        assert!(result.contains("- Second learning (from phase: spec)"));
    }

    #[test]
    fn test_format_memories_with_categories() {
        let entries = vec![
            MemoryEntry::new(1, "General learning".to_string(), None, None),
            MemoryEntry::new(
                2,
                "Code pattern".to_string(),
                Some("code_style".to_string()),
                None,
            ),
            MemoryEntry::new(
                3,
                "Another pattern".to_string(),
                Some("code_style".to_string()),
                None,
            ),
        ];

        let result = format_memories(&entries).unwrap();
        assert!(result.contains("- General learning"));
        assert!(result.contains("### Code Style"));
        assert!(result.contains("- Code pattern"));
        assert!(result.contains("- Another pattern"));
    }

    #[test]
    fn test_to_title_case() {
        assert_eq!(to_title_case("code_style"), "Code Style");
        assert_eq!(to_title_case("project-structure"), "Project Structure");
        assert_eq!(to_title_case("simple"), "Simple");
    }
}
