//! Tauri commands：规则引擎薄封装（对应 spec §3.4）。

use std::path::{Path, PathBuf};

use tauri::Manager;
use tauri::State;

use crate::rules::engine::{Chapter, Detail, ParseResult, RuleExecError, RuleInput, SearchItem};
use crate::rules::schema::{
    file_stem_id, validate_manifest, LoadedRule, RuleFileFormat, RuleLoadError, RuleManifest,
    RuleOrigin, RuleStatus,
};
use crate::rules::{RuleEngine, RuleEngineState};

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

/// 搜索。
///
/// 并发 scope 契约（Kimi K3 复审第 3 项）：spec §3.4 的 `rules_search` 签名不含 `scope`
/// 参数，本命令用 `per_call_token` 生成一个**仅覆盖单次调用生命周期**的取消 token
/// （不注册进 scope 表）——同一规则上的并发 search（翻页 page=1 与 page=2）互不取消。
/// 前端若需取消整套源切换，通过 `rules_parse` 传入的 `play:{contentId}` 作用域 +
/// `rules_cancel_scope` 实现——搜索/详情/章节属过程性子操作，随其所属的 play 作用域
/// 一起被取消（switchSource 的 `cancelScope(scope)` 会先取消 play 作用域）。
#[tauri::command]
pub async fn rules_search(
    state: State<'_, RuleEngineState>,
    rule_id: String,
    keyword: String,
    page: u32,
) -> Result<Vec<SearchItem>, RuleExecError> {
    let token = state.0.per_call_token();
    state.0.search(&rule_id, &keyword, page, token).await
}

/// 详情。scope 契约同 `rules_search`（每次调用独立 token，不取消同规则其他调用）。
#[tauri::command]
pub async fn rules_detail(
    state: State<'_, RuleEngineState>,
    rule_id: String,
    url: String,
) -> Result<Detail, RuleExecError> {
    let token = state.0.per_call_token();
    state.0.detail(&rule_id, &url, token).await
}

/// 章节列表。scope 契约同 `rules_search`（每次调用独立 token，不取消同规则其他调用）。
#[tauri::command]
pub async fn rules_chapters(
    state: State<'_, RuleEngineState>,
    rule_id: String,
    detail_url: String,
) -> Result<Vec<Chapter>, RuleExecError> {
    let token = state.0.per_call_token();
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

/// 选取不与已注册 id 或磁盘文件冲突的自定义规则落盘 id。
///
/// 稳定 id = 源文件 stem（Kimi K3 复审第 1 项）：落盘名 = stem，重启后
/// `rules_load_all` 以同一文件重载时 id 恒等；`rules_remove_custom` 按同一 id
/// 仍能定位并删除该文件——id 若随机生成，重启后 id 与文件名对不上，删除链路
/// 断裂会让规则「复活」。
///
/// 静默覆盖防护（Kimi K3 复审第 4 项）：`rules_import` 若直接写 `custom_rules/{stem}.json`，
/// 导入两个同名 stem 的规则文件会静默覆盖前一个（数据丢失 + 注册表条目被替换），
/// 同名 stem 还可能遮蔽内置规则。因此在写入前检测：目标文件已存在 **或** 该 id 已注册时，
/// 自动追加 `-2`/`-3`… 后缀形成新 id——落盘名即 id，「id = 文件名 stem」的不变量
/// 对后缀名同样成立（重启重载、删除链路均不受影响），且绝不静默覆盖。
pub(crate) fn unique_custom_rule_id(engine: &RuleEngine, stem: &str, dir: &Path) -> String {
    let mut id = stem.to_string();
    let mut n = 2u32;
    while engine.is_registered(&id) || dir.join(format!("{id}.json")).exists() {
        id = format!("{stem}-{n}");
        n += 1;
    }
    id
}

/// 导入规则文件的核心流程（`dir` 为自定义规则落盘目录，可注入便于测试）。
///
/// 读取 → 解析 → schema 校验 → 编译 → 选取不冲突的 id 落盘 → 注册。
/// 任何一步失败返回结构化 [`RuleLoadError`]（前端展示具体校验错误）。
pub(crate) async fn import_rule_to_dir(
    engine: &RuleEngine,
    path: &Path,
    dir: &Path,
) -> Result<LoadedRule, RuleLoadError> {
    let format = RuleFileFormat::from_path(path)
        .ok_or_else(|| RuleLoadError::schema("不支持的文件格式（仅 .json/.yaml/.yml）"))?;
    let text = std::fs::read_to_string(path)
        .map_err(|e| RuleLoadError::schema(format!("读取规则文件失败: {e}")))?;
    let manifest = RuleManifest::from_str(&text, format)?;
    validate_manifest(&manifest)?;
    engine.compile_manifest(&manifest).await?;

    let stem = file_stem_id(path);
    let id = unique_custom_rule_id(engine, &stem, dir);
    let target = dir.join(format!("{id}.json"));
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

/// 导入本地规则文件（前端弹文件对话框拿到路径后传入）。
#[tauri::command]
pub async fn rules_import(
    state: State<'_, RuleEngineState>,
    path: String,
) -> Result<LoadedRule, RuleLoadError> {
    let engine = &state.0;
    import_rule_to_dir(engine, Path::new(&path), &custom_rules_dir()).await
}

/// 删除自定义规则文件：对 json/yaml/yml 同 stem 文件都尝试删除（存在才删）。
///
/// Kimi K3 复审第 5 项：`discover_rule_inputs` 从自定义目录加载 `.json/.yaml/.yml`
/// （id = 文件名 stem，见 [`file_stem_id`]）。若删除只清 `{id}.json`，同名手动放置的
/// `.yaml`/`.yml` 规则会在下次 `rules_load_all` 时以同一 stem id 重新注册，规则「复活」。
/// 因此对三种扩展名的同 stem 文件都尝试删除，杜绝手动 YAML 规则残留复活。
pub(crate) fn remove_custom_rule_files(dir: &Path, rule_id: &str) -> Result<(), String> {
    for ext in ["json", "yaml", "yml"] {
        let file = dir.join(format!("{rule_id}.{ext}"));
        if file.exists() {
            std::fs::remove_file(&file).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// 删除自定义规则（内置规则拒绝删除）。同 stem 的 json/yaml/yml 文件一并清除。
#[tauri::command]
pub async fn rules_remove_custom(
    state: State<'_, RuleEngineState>,
    rule_id: String,
) -> Result<(), String> {
    let engine = &state.0;
    engine.remove_rule(&rule_id)?;
    remove_custom_rule_files(&custom_rules_dir(), &rule_id)
}

/// 导出全部规则（内置 + 自定义）为 JSON 数组到指定路径。
#[tauri::command]
pub async fn rules_export(state: State<'_, RuleEngineState>, path: String) -> Result<u32, String> {
    let manifests = state.0.all_manifests();
    let json = serde_json::to_string_pretty(&manifests).map_err(|e| e.to_string())?;
    // 目标父目录不存在时先创建，避免导出静默失败（DeepSeek 审核建议）。
    if let Some(parent) = std::path::Path::new(&path).parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建导出目录失败: {e}"))?;
        }
    }
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(manifests.len() as u32)
}
