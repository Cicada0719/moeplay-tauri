//! Provider-agnostic AI gateway foundation.
//!
//! This module is intentionally isolated from Tauri commands and persistence.
//! Consumers must explicitly register it from `lib.rs` and inject transport,
//! secret lookup, and durable task storage at the integration boundary.

pub mod change_set;
pub mod contracts;
pub mod endpoint;
pub mod error;
pub mod governance;
pub mod ollama;
pub mod openai_compatible;
pub mod orchestrator;
pub mod prompts;
pub mod provider;
pub mod redaction;
pub mod schema;
pub mod transport;

pub use change_set::*;
pub use contracts::*;
pub use endpoint::*;
pub use error::*;
pub use governance::*;
pub use ollama::*;
pub use openai_compatible::*;
pub use orchestrator::*;
pub use prompts::*;
pub use provider::*;
pub use redaction::*;
pub use schema::*;
pub use transport::*;
