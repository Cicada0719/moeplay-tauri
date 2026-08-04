//! 规则系统：kazumi 兼容的沙箱化规则执行引擎。
//!
//! - [`schema`]：规则清单（RuleManifest）与加载/校验。
//! - [`sandbox`]：QuickJS 沙箱（无 std/os，注入 fetch/console）。
//! - [`engine`]：规则引擎（worker 线程池 + CancellationToken 竞态取消）。

pub mod engine;
pub mod sandbox;
pub mod schema;

pub use engine::RuleEngine;

/// Tauri 管理状态：规则引擎实例。
pub struct RuleEngineState(pub RuleEngine);

#[cfg(test)]
mod tests;
