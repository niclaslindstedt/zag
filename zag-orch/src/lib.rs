//! Orchestration library for zag — multi-session coordination for AI coding agents.
//!
//! This crate provides the orchestration primitives: spawn, wait, collect, pipe,
//! status, events, cancel, summary, watch, subscribe, retry, gc, and more.
//! It depends on `zag` (zag-lib) for shared types (session_log, config, session,
//! process_store) and for agent execution (AgentBuilder in pipe).

pub mod types;
pub mod util;

pub mod cancel;
pub mod collect;
pub mod env;
pub mod events;
pub mod gc;
pub mod lifecycle;
pub mod listen;
pub mod log_cmd;
pub mod output_cmd;
pub mod pipe;
pub mod ps;
pub mod retry;
pub mod search;
pub mod spawn;
pub mod status;
pub mod subscribe;
pub mod summary;
pub mod wait;
pub mod watch;
pub mod whoami;
