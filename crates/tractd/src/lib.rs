//! tractd library surface.
//!
//! The binary entry point lives in `main.rs`; this crate's library target
//! exists so external tools (e.g. the eval harness in `tools/eval`) can
//! call into `Orchestrator::extract_from_html` without re-implementing it.

pub mod client;
pub mod handler;
pub mod ipc;
pub mod orchestrator;
pub mod server;
pub mod state;
