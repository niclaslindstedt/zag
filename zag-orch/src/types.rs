//! Shared types for orchestration commands.

/// Session metadata for discovery (name, description, tags).
#[derive(Clone, Default)]
pub struct SessionMetadata {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
}
