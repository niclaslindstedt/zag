//! Unified Rust interface for zag — AI coding agent orchestration.
//!
//! This crate re-exports [`zag-agent`] (core agent library) and [`zag-orch`]
//! (orchestration primitives) under a single `zag` package name.
//!
//! # Usage
//!
//! ```rust,ignore
//! use zag::builder::AgentBuilder;
//! use zag::orch::spawn;
//! ```

// Re-export zag-agent at the top level so users can write `use zag::builder::AgentBuilder`, etc.
pub use zag_agent::*;

/// Orchestration primitives: spawn, wait, collect, pipe, status, events, cancel, and more.
pub mod orch {
    pub use zag_orch::*;
}

/// Network server: remote access to AI agent orchestration over HTTP/WebSocket.
pub mod serve {
    pub use zag_serve::*;
}
