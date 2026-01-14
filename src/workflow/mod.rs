//! Workflow engine for orchestrating multi-phase AI agent sessions.
//!
//! # Overview
//!
//! The workflow system allows defining multi-phase workflows where each phase
//! is executed by an AI agent. Phases can:
//! - Run once
//! - Iterate over items in a JSON file
//! - Have nested phases that run within iterations
//!
//! # State Management
//!
//! Workflows use the filesystem for state management. Each workflow run creates
//! a state directory under `.agent/state/<workflow>/<run_id>/` where agents
//! store their outputs and read inputs from previous phases.
//!
//! # Example: Software Workflow
//!
//! The embedded "software" workflow implements epic-based development:
//! 1. Write specification
//! 2. Plan epics (features)
//! 3. For each epic:
//!    - Create tickets
//!    - For each ticket:
//!      - Implement
//!      - Review (creates follow-ups)
//!      - Complete follow-ups before next ticket
//!
//! # Creating Custom Workflows
//!
//! Place JSON workflow files in `~/.agent/workflows/` to override embedded
//! workflows or create new ones. See the documentation for the JSON schema.

pub mod definitions;
pub mod manage;
pub mod engine;
pub mod loader;
pub mod phase;
pub mod state;
pub mod template;
pub mod types;
pub mod variables;

pub use engine::WorkflowEngine;
