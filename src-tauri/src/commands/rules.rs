//! Tauri commands：规则引擎薄封装（对应 spec §3.4）。

use std::path::{Path, PathBuf};

use tauri::Manager;
use tauri::State;

use crate::rules::engine::{Chapter, Detail, ParseResult, RuleExecError, RuleInput, SearchItem};
use crate::rules::health::{self, HealthProbeResult, SourceHealthInfo};
use crate::rules::schema::{
    file_stem_id, validate_manifest, LoadedRule, RuleFileFormat, RuleLoadError, RuleManifest,
    RuleOrigin, RuleStatus,
};
use crate::rules::update::{self, RulesMetaInfo, UpdateOutcome};
use crate::rules::{collect_rule_inputs, RuleEngine, RuleEngineState};

/// 内置规则目录（spec Step 3.7）：优先 `$APPDATA/rules-cache/current`（远端热更新缓存），
/// 否则打包资源目录 `resources/rules/`——这是与子任务 1 规则加载入口的唯一集成点。
fn builtin_rules_dir(app: &tauri::AppHandle) -> PathBuf {
    update::resolve_rules_dir(app)
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
///
/// 规则按类型分目录存放（`resources/rules/{anime,manga,novel}`），热更新缓存亦然，
/// 因此递归扫描子目录；`manifest.json` 是清单而非规则，跳过。
fn discover_rule_inputs(app: &tauri::AppHandle) -> Vec<RuleInput> {
    let mut inputs = Vec::new();
    collect_rule_inputs(&builtin_rules_dir(app), RuleOrigin::Builtin, &mut inputs);
    collect_rule_inputs(&custom_rules_dir(), RuleOrigin::Custom, &mut inputs);
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
/// `invocation` 契约（Kimi K3 复审第 7 项）：`switchSource` 每次调用生成**独立** invocation
/// scope（如 `play:{contentId}:{seq}`），search/chapters/parse 全程绑定该 scope——新调用通过
/// `cancel_scope(旧 invocation)` 把旧调用整体作废；旧调用迟到的 parse 因 scope 唯一无法取消
/// 最新调用的 token。`invocation` 为空（翻页等非换源场景）时退化为 per-call token，同规则
/// 并发调用（page=1 与 page=2）互不取消。
#[tauri::command]
pub async fn rules_search(
    state: State<'_, RuleEngineState>,
    rule_id: String,
    keyword: String,
    page: u32,
    invocation: String,
) -> Result<Vec<SearchItem>, RuleExecError> {
    let token = state.0.invocation_token(&invocation);
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

/// 章节列表。`invocation` 契约同 `rules_search`（绑定 switchSource 调用级 scope）。
#[tauri::command]
pub async fn rules_chapters(
    state: State<'_, RuleEngineState>,
    rule_id: String,
    detail_url: String,
    invocation: String,
) -> Result<Vec<Chapter>, RuleExecError> {
    let token = state.0.invocation_token(&invocation);
    state.0.chapters(&rule_id, &detail_url, token).await
}

/// 解析播放地址。`scope` 由前端传入：`switchSource` 传入本次调用的 invocation scope
/// （如 "play:{contentId}:{seq}"），实现 FR-02「仅末次调用生效」——scope 按调用唯一，
/// 旧调用迟到的 parse 不会取消最新调用已注册的 token（Kimi K3 复审第 7 项）。
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

/// 删除自定义规则的核心流程（`dir` 为自定义规则落盘目录，可注入便于测试）。
///
/// 顺序契约（Kimi K3 复审第 5 项）：**先删文件、再移除注册表**。若先移除注册表而
/// 文件删除失败，会留下「注册表已删、文件仍在」的状态不一致——残留文件会在下次
/// `rules_load_all` 时以同 stem id 复活。文件删除失败时注册表条目保留，状态一致。
pub(crate) fn remove_custom_rule(
    engine: &RuleEngine,
    dir: &Path,
    rule_id: &str,
) -> Result<(), String> {
    remove_custom_rule_files(dir, rule_id)?;
    engine.remove_rule(rule_id)
}

/// 删除自定义规则（内置规则拒绝删除）。同 stem 的 json/yaml/yml 文件一并清除。
#[tauri::command]
pub async fn rules_remove_custom(
    state: State<'_, RuleEngineState>,
    rule_id: String,
) -> Result<(), String> {
    let engine = &state.0;
    remove_custom_rule(engine, &custom_rules_dir(), &rule_id)
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

/// 规则包元信息（设置页展示版本/来源/更新时间）。
#[tauri::command]
pub async fn rules_get_meta(app: tauri::AppHandle) -> Result<RulesMetaInfo, String> {
    update::rules_meta_info(&app)
}

/// 检查并更新规则包（启动时自动调用 + 设置页"检查更新"按钮）。
///
/// `force=true` 跳过 24h 节流。网络/签名/应用失败一律回退本地缓存（`FallbackCached`），
/// 不向用户报错（FR-04）。
#[tauri::command]
pub async fn rules_check_and_update(
    app: tauri::AppHandle,
    state: State<'_, RuleEngineState>,
    force: bool,
) -> Result<UpdateOutcome, String> {
    let outcome = update::check_and_update(&app, force).await?;
    // 更新成功 → 以新规则目录热重载引擎注册表。
    if matches!(outcome.status, update::UpdateStatus::Updated) {
        let dir = update::resolve_rules_dir(&app);
        let loaded = crate::rules::reload_all(&state.0, &dir).await;
        let ready = loaded
            .iter()
            .filter(|r| r.status == RuleStatus::Ready)
            .count();
        tracing::info!("热更新后重载规则: Ready {ready}/{}", loaded.len());
    }
    Ok(outcome)
}

/// 立即健康检查（`None` 表示全量探测；并发，单源 10s 超时）。
#[tauri::command]
pub async fn rules_probe_health(
    app: tauri::AppHandle,
    state: State<'_, RuleEngineState>,
    source_ids: Option<Vec<String>>,
) -> Result<Vec<HealthProbeResult>, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("应用数据目录不可用: {e}"))?;
    let rules_dir = update::resolve_rules_dir(&app);
    Ok(health::probe_all(&state.0, &app_data, &rules_dir, source_ids).await)
}

/// 读取持久化的源健康状态（源列表页展示；不触发探测）。
#[tauri::command]
pub async fn rules_get_health(app: tauri::AppHandle) -> Result<Vec<SourceHealthInfo>, String> {
    health::get_health_info(&app)
}
