//! Tauri commands：规则引擎薄封装（对应 spec §3.4）。

use std::path::PathBuf;

use tauri::Manager;
use tauri::State;
use uuid::Uuid;

use crate::rules::engine::{
    Detail, ParseResult, RuleExecError, RuleInput, SearchItem, Chapter,
};
use crate::rules::schema::{
    validate_manifest, LoadedRule, RuleFileFormat, RuleLoadError, RuleManifest, RuleOrigin,
    RuleStatus,
};
use crate::rules::RuleEngineState;

/// 内置规则目录：优先资源目录，其次源码 resources/rules，最后创建空目录。
fn builtin_rules_dir(app: &tauri::AppHandle) -> PathBuf {
    if let Ok(dir) = app.path().resource_dir() {
        let candidate = dir.join("rules");
        if candidate.exists() {
            return candidate;
        }
    }
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("rules");
    if dev.exists() {
        return dev;
    }
    let fallback = dirs::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("moeplay")
        .join("rules");
    if std::fs::create_dir_all(&fallback).is_ok() {
        tracing::warn!("内置规则目录不存在，已创建空目录: {}", fallback.display());
    }
    fallback
}

/// 自定义规则目录。
fn custom_rules_dir() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("moeplay")
        .join("custom_rules");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        tracing::warn!("创建自定义规则目录失败: {e}");
    }
    dir
}

/// 扫描内置 + 自定义规则文件，构造加载输入。
fn discover_rule_inputs(app: &tauri::AppHandle) -> Vec<RuleInput> {
    let mut inputs = Vec::new();
    for (path, origin) in [
        (builtin_rules_dir(app), RuleOrigin::Builtin),
        (custom_rules_dir(), RuleOrigin::Custom),
    ] {
        if let Ok(entries) = std::fs::read_dir(&path) {
            for entry in entries.flatten() {
                let file = entry.path();
                if RuleFileFormat::from_path(&file).is_some() {
                    inputs.push(RuleInput::File { path: file, origin });
                }
            }
        }
    }
    inputs
}

/// 加载全部规则（内置 + 自定义），返回含 Invalid 项的完整列表。
#[tauri::command]
pub async fn rules_load_all(
    app: tauri::AppHandle,
    state: State<'_, RuleEngineState>,
) -> Result<Vec<LoadedRule>, String> {
    let inputs = discover_rule_inputs(&app);
    Ok(state.0.load_rules(inputs).await)
}

/// 搜索
#[tauri::command]
pub async fn rules_search(
    state: State<'_, RuleEngineState>,
    rule_id: String,
    keyword: String,
    page: u32,
) -> Result<Vec<SearchItem>, RuleExecError> {
    let scope = format!("search:{rule_id}");
    let token = state.0.new_scope_token(&scope);
    state.0.search(&rule_id, &keyword, page, token).await
}

/// 详情
#[tauri::command]
pub async fn rules_detail(
    state: State<'_, RuleEngineState>,
    rule_id: String,
    url: String,
) -> Result<Detail, RuleExecError> {
    let scope = format!("detail:{rule_id}");
    let token = state.0.new_scope_token(&scope);
    state.0.detail(&rule_id, &url, token).await
}

/// 章节列表
#[tauri::command]
pub async fn rules_chapters(
    state: State<'_, RuleEngineState>,
    rule_id: String,
    detail_url: String,
) -> Result<Vec<Chapter>, RuleExecError> {
    let scope = format!("chapters:{rule_id}");
    let token = state.0.new_scope_token(&scope);
    state.0.chapters(&rule_id, &detail_url, token).await
}

/// 解析播放地址。`scope` 由前端传入（如 "play:{contentId}"），实现 FR-02 取消语义。
#[tauri::command]
pub async fn rules_parse(
    state: State<'_, RuleEngineState>,
    rule_id: String,
    chapter_url: String,
    scope: String,
) -> Result<ParseResult, RuleExecError> {
    let token = state.0.new_scope_token(&scope);
    state.0.parse(&rule_id, &chapter_url, token).await
}

/// 取消指定 scope 下所有未完成任务。
#[tauri::command]
pub async fn rules_cancel_scope(
    state: State<'_, RuleEngineState>,
    scope: String,
) -> Result<(), String> {
    state.0.cancel_scope(&scope);
    Ok(())
}

/// 导入本地规则文件（前端弹文件对话框拿到路径后传入）。
#[tauri::command]
pub async fn rules_import(
    state: State<'_, RuleEngineState>,
    path: String,
) -> Result<LoadedRule, RuleLoadError> {
    let engine = &state.0;
    let path_buf = PathBuf::from(&path);
    let format = RuleFileFormat::from_path(&path_buf)
        .ok_or_else(|| RuleLoadError::schema("不支持的文件格式（仅 .json/.yaml/.yml）"))?;
    let text = std::fs::read_to_string(&path_buf)
        .map_err(|e| RuleLoadError::schema(format!("读取规则文件失败: {e}")))?;
    let manifest = RuleManifest::from_str(&text, format)?;
    validate_manifest(&manifest)?;
    engine.compile_manifest(&manifest).await?;

    let id = Uuid::new_v4().to_string();
    let target = custom_rules_dir().join(format!("{id}.json"));
    let json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| RuleLoadError::schema(format!("序列化规则失败: {e}")))?;
    std::fs::write(&target, json)
        .map_err(|e| RuleLoadError::schema(format!("写入自定义规则失败: {e}")))?;

    let loaded = LoadedRule {
        id: id.clone(),
        manifest,
        origin: RuleOrigin::Custom,
        status: RuleStatus::Ready,
        error: None,
    };
    engine.register_loaded(loaded.clone());
    Ok(loaded)
}

/// 删除自定义规则（内置规则拒绝删除）。
#[tauri::command]
pub async fn rules_remove_custom(
    state: State<'_, RuleEngineState>,
    rule_id: String,
) -> Result<(), String> {
    let engine = &state.0;
    engine.remove_rule(&rule_id)?;
    let file = custom_rules_dir().join(format!("{rule_id}.json"));
    if file.exists() {
        std::fs::remove_file(&file).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 导出全部规则（内置 + 自定义）为 JSON 数组到指定路径。
#[tauri::command]
pub async fn rules_export(
    state: State<'_, RuleEngineState>,
    path: String,
) -> Result<u32, String> {
    let manifests = state.0.all_manifests();
    let json = serde_json::to_string_pretty(&manifests).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(manifests.len() as u32)
}
