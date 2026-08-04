//! 规则系统：kazumi 兼容的沙箱化规则执行引擎。
//!
//! - [`schema`]：规则清单（RuleManifest）与加载/校验。
//! - [`sandbox`]：QuickJS 沙箱（无 std/os，注入 fetch/console）。
//! - [`engine`]：规则引擎（worker 线程池 + CancellationToken 竞态取消）。
//! - [`manifest_gen`]：构建期规则清单生成（build.rs 以 `#[path]` 复用，独立于 crate）。
//! - [`update`]：规则包热更新（拉取 → ed25519 校验 → 原子替换 → 回退缓存）。
//! - [`health`]：源健康检查（探测 → 状态推导 → 持久化）。
//!
//! ## 与子任务 1 的适配层
//!
//! spec §5 假设引擎提供 `load_from_dir` / `reload_all` / `execute(action)` /
//! `new_headless` / `list_rules` 等接口；实际子任务 1 引擎导出的是
//! `RuleEngine::load_rules(Vec<RuleInput>)`、`search/detail/chapters/parse`、
//! `new(http)` 等。这里在模块层提供等价适配函数，**不修改** `engine.rs`/`schema.rs` 的公共 API。

pub mod engine;
pub mod health;
pub mod manifest_gen;
pub mod sandbox;
pub mod schema;
pub mod update;

use std::path::Path;
use std::sync::Arc;

pub use engine::RuleEngine;

use crate::rules::engine::RuleInput;
use crate::rules::schema::{LoadedRule, RuleFileFormat, RuleOrigin};

/// Tauri 管理状态：规则引擎实例（`Arc` 便于后台探测任务/热更新持有克隆）。
pub struct RuleEngineState(pub Arc<RuleEngine>);

/// 构建一个无 GUI 依赖的引擎（CI `rules-health` headless bin 用）。
pub fn new_headless() -> RuleEngine {
    RuleEngine::new(
        reqwest::Client::builder()
            .user_agent(concat!("moeplay/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("failed to build headless rule engine http client"),
    )
}

/// 从目录加载全部规则（递归扫描 `.json/.yaml/.yml`，跳过 `manifest.json`），origin=Builtin。
pub async fn load_rules_from_dir(engine: &RuleEngine, dir: &Path) -> Vec<LoadedRule> {
    let mut inputs = Vec::new();
    collect_rule_inputs(dir, RuleOrigin::Builtin, &mut inputs);
    engine.load_rules(inputs).await
}

/// 热更新后重载（`apply_update` 成功后调用）：以新目录内容 upsert 引擎注册表。
pub async fn reload_all(engine: &RuleEngine, dir: &Path) -> Vec<LoadedRule> {
    load_rules_from_dir(engine, dir).await
}

/// 递归收集规则文件输入（跳过 `manifest.json`——它是清单而非规则）。
pub fn collect_rule_inputs(dir: &Path, origin: RuleOrigin, out: &mut Vec<RuleInput>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_rule_inputs(&path, origin, out);
            } else {
                let is_manifest = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n == "manifest.json")
                    .unwrap_or(false);
                if is_manifest {
                    continue;
                }
                if RuleFileFormat::from_path(&path).is_some() {
                    out.push(RuleInput::File { path, origin });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
