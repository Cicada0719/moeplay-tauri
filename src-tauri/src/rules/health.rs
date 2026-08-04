//! 源健康检查（spec task-02 Step 4）：搜索探测 → 状态推导 → 持久化。
//!
//! - 探测：对每条规则的 `search` 动作执行一次真实搜索（`probeKeyword`），带 10s 超时；
//! - 判定：无异常且返回 ≥1 条 → 成功；空数组/异常/超时 → 失败；
//! - 持久化：`$APPDATA/rules-health.json`，每条源保留最近 10 次探测记录；
//! - 状态：连续失败 ≥3 → `Abnormal`，≥1 → `Degraded`，否则 `Healthy`，无记录 → `Unknown`。

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::rules::engine::{RuleEngine, RuleExecError};
use crate::rules::update::{self, RuleManifest};

/// 单源探测超时（spec Step 4.2，10s）。
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);
/// 并发探测上限（spec Step 4.3，8）。
const MAX_CONCURRENT_PROBES: usize = 8;
/// 结果队列保留条数。
const MAX_RESULT_HISTORY: usize = 10;

/// 健康状态。
///
/// 序列化为 PascalCase（`Healthy`/`Degraded`/`Abnormal`/`Unknown`），与 spec §3.5
/// 前端接口 `status: 'Healthy' | 'Degraded' | 'Abnormal' | 'Unknown'` 一一对应。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Abnormal,
    Unknown,
}

/// 单次探测记录。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeRecord {
    pub ok: bool,
    pub latency_ms: u64,
    pub checked_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 单源健康记录（持久化到 `rules-health.json`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthRecord {
    pub results: VecDeque<ProbeRecord>,
    pub consecutive_failures: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_checked_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_latency_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

impl HealthRecord {
    fn new() -> Self {
        Self {
            results: VecDeque::new(),
            consecutive_failures: 0,
            last_checked_at: None,
            last_latency_ms: None,
            last_error: None,
        }
    }
}

/// 前端 `rules_get_health` 返回的单源健康信息。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceHealthInfo {
    pub source_id: String,
    pub status: HealthStatus,
    pub consecutive_failures: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_checked_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

/// 单源探测结果（`rules_probe_health` 返回）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthProbeResult {
    pub source_id: String,
    pub ok: bool,
    pub latency_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 待探测源（id + 探测关键词）。
#[derive(Debug, Clone)]
pub struct DiscoveredRule {
    pub id: String,
    pub keyword: String,
}

// ── 持久化 ────────────────────────────────────────────────────────────────

/// 加载健康文件；损坏/缺失时降级为空 map（不 panic，spec §6.1 测试 13）。
pub fn load_health(path: &Path) -> HashMap<String, HealthRecord> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => HashMap::new(),
    }
}

/// 保存健康文件（父目录不存在时创建）。
pub fn save_health(path: &Path, map: &HashMap<String, HealthRecord>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    let json = serde_json::to_string_pretty(map).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

// ── 探测 ──────────────────────────────────────────────────────────────────

/// 单源探测（AppHandle 无关核心，CI `rules-health` bin 复用）。
///
/// 成功判定：无异常且 `search` 返回数组长度 ≥ 1；空数组视为失败。
pub async fn probe_one(engine: &RuleEngine, rule_id: &str, keyword: &str) -> HealthProbeResult {
    probe_one_with_timeout(engine, rule_id, keyword, PROBE_TIMEOUT).await
}

/// 单源探测（超时可注入，测试用短超时避免真实 10s 等待；生产经 [`probe_one`] 用 10s）。
async fn probe_one_with_timeout(
    engine: &RuleEngine,
    rule_id: &str,
    keyword: &str,
    timeout: Duration,
) -> HealthProbeResult {
    let started = std::time::Instant::now();
    let token = engine.per_call_token();
    let result = tokio::time::timeout(timeout, engine.search(rule_id, keyword, 1, token)).await;
    let latency_ms = started.elapsed().as_millis() as u64;
    match result {
        Ok(Ok(items)) => {
            if items.is_empty() {
                HealthProbeResult {
                    source_id: rule_id.to_string(),
                    ok: false,
                    latency_ms,
                    error: Some("搜索结果为空（0 条）".to_string()),
                }
            } else {
                HealthProbeResult {
                    source_id: rule_id.to_string(),
                    ok: true,
                    latency_ms,
                    error: None,
                }
            }
        }
        Ok(Err(e)) => HealthProbeResult {
            source_id: rule_id.to_string(),
            ok: false,
            latency_ms,
            error: Some(describe_exec_error(&e)),
        },
        Err(_elapsed) => HealthProbeResult {
            source_id: rule_id.to_string(),
            ok: false,
            latency_ms,
            error: Some(format!("探测超时（>{timeout:?}）")),
        },
    }
}

/// 从规则目录发现待探测源：优先读 `manifest.json`，缺失/损坏则回退扫描规则文件。
pub fn discover_rules(rules_dir: &Path) -> Vec<DiscoveredRule> {
    let mut out = Vec::new();
    let manifest_path = rules_dir.join("manifest.json");
    if let Ok(text) = std::fs::read_to_string(&manifest_path) {
        if let Ok(m) = serde_json::from_str::<RuleManifest>(&text) {
            for e in m.rules {
                if let Some(kw) = e.probe_keyword.as_deref() {
                    if !kw.is_empty() {
                        out.push(DiscoveredRule {
                            id: e.id.clone(),
                            keyword: kw.to_string(),
                        });
                    }
                }
            }
            if !out.is_empty() {
                return out;
            }
        }
    }
    scan_rule_files(rules_dir, &mut out);
    out
}

/// 只取规则 id（`rules_get_health` 关联当前规则列表用）。
pub fn discover_rule_ids(rules_dir: &Path) -> Vec<String> {
    discover_rules(rules_dir)
        .into_iter()
        .map(|r| r.id)
        .collect()
}

fn scan_rule_files(dir: &Path, out: &mut Vec<DiscoveredRule>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_rule_files(&path, out);
            } else {
                let is_manifest = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n == "manifest.json")
                    .unwrap_or(false);
                let is_rule_file = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| matches!(e, "json" | "yaml" | "yml"))
                    .unwrap_or(false);
                if is_manifest || !is_rule_file {
                    continue;
                }
                if let Ok(text) = std::fs::read_to_string(&path) {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                        // id 取文件名 stem（与引擎 file_stem_id 一致）
                        let id = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or_default()
                            .to_string();
                        let kw = v
                            .get("probeKeyword")
                            .and_then(|x| x.as_str())
                            .unwrap_or_default()
                            .to_string();
                        if !id.is_empty() && !kw.is_empty() {
                            out.push(DiscoveredRule { id, keyword: kw });
                        }
                    }
                }
            }
        }
    }
}

/// 全量/子集探测：并发（上限 8），逐源更新 `HealthRecord` 并落盘。
pub async fn probe_all(
    engine: &RuleEngine,
    app_data: &Path,
    rules_dir: &Path,
    source_ids: Option<Vec<String>>,
) -> Vec<HealthProbeResult> {
    probe_all_with_timeout(engine, app_data, rules_dir, source_ids, PROBE_TIMEOUT).await
}

/// 探测核心（超时可注入，测试用短超时验证「单源超时不拖垮其余源」）。
async fn probe_all_with_timeout(
    engine: &RuleEngine,
    app_data: &Path,
    rules_dir: &Path,
    source_ids: Option<Vec<String>>,
    timeout: Duration,
) -> Vec<HealthProbeResult> {
    let mut targets = discover_rules(rules_dir);
    if let Some(ids) = source_ids {
        let set: HashSet<String> = ids.into_iter().collect();
        targets.retain(|t| set.contains(&t.id));
    }
    let health_path = app_data.join("rules-health.json");
    let mut health = load_health(&health_path);
    let now = now_ts();

    let results = run_probes(targets, MAX_CONCURRENT_PROBES, |t| async move {
        probe_one_with_timeout(engine, &t.id, &t.keyword, timeout).await
    })
    .await;

    for r in &results {
        let rec = health
            .entry(r.source_id.clone())
            .or_insert_with(HealthRecord::new);
        record_append(rec, r, now);
    }
    let _ = save_health(&health_path, &health);
    results
}

/// 把一次探测结果并入健康记录：追加记录（最多保留 `MAX_RESULT_HISTORY` 条）、
/// 更新连续失败计数与最近状态。独立抽出便于单测（spec §6.1 测试 13）。
fn record_append(rec: &mut HealthRecord, r: &HealthProbeResult, now: i64) {
    rec.results.push_back(ProbeRecord {
        ok: r.ok,
        latency_ms: r.latency_ms,
        checked_at: now,
        error: r.error.clone(),
    });
    while rec.results.len() > MAX_RESULT_HISTORY {
        rec.results.pop_front();
    }
    rec.last_checked_at = Some(now);
    rec.last_latency_ms = Some(r.latency_ms);
    if r.ok {
        if rec.consecutive_failures > 0 {
            tracing::info!("源 {} 健康恢复（consecutive_failures -> 0）", r.source_id);
        }
        rec.consecutive_failures = 0;
        rec.last_error = None;
    } else {
        rec.consecutive_failures = rec.consecutive_failures.saturating_add(1);
        rec.last_error = r.error.clone();
        if rec.consecutive_failures >= 3 {
            tracing::info!("源 {} 健康状态变为 Abnormal", r.source_id);
        } else {
            tracing::debug!(
                "源 {} 探测失败（第 {} 次）: {}",
                r.source_id,
                rec.consecutive_failures,
                r.error.as_deref().unwrap_or("未知错误")
            );
        }
    }
}

/// 并发执行探测（上限 `concurrency`）。独立抽出便于测试并发上限。
async fn run_probes<F, Fut>(
    targets: Vec<DiscoveredRule>,
    concurrency: usize,
    probe: F,
) -> Vec<HealthProbeResult>
where
    F: Fn(DiscoveredRule) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = HealthProbeResult> + Send,
{
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(concurrency));
    let futs = targets.into_iter().map(|t| {
        let sem = sem.clone();
        let p = &probe;
        async move {
            let _permit = sem
                .clone()
                .acquire_owned()
                .await
                .map_err(|_| ())
                .expect("探测并发门闩已关闭");
            (p)(t).await
        }
    });
    futures_util::future::join_all(futs).await
}

/// 状态推导（spec Step 4.4）。
pub fn derive_status(rec: Option<&HealthRecord>) -> HealthStatus {
    match rec {
        None => HealthStatus::Unknown,
        Some(r) if r.consecutive_failures >= 3 => HealthStatus::Abnormal,
        Some(r) if r.consecutive_failures >= 1 => HealthStatus::Degraded,
        Some(_) => HealthStatus::Healthy,
    }
}

/// `rules_get_health` 命令实现：加载健康文件，对当前规则列表逐源生成 `SourceHealthInfo`。
pub fn get_health_info(app: &tauri::AppHandle) -> Result<Vec<SourceHealthInfo>, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("应用数据目录不可用: {e}"))?;
    let rules_dir = update::resolve_rules_dir(app);
    let ids = discover_rule_ids(&rules_dir);
    let health = load_health(&app_data.join("rules-health.json"));
    Ok(ids
        .into_iter()
        .map(|id| {
            let rec = health.get(&id);
            SourceHealthInfo {
                source_id: id.clone(),
                status: derive_status(rec),
                consecutive_failures: rec.map(|r| r.consecutive_failures).unwrap_or(0),
                last_checked_at: rec.and_then(|r| r.last_checked_at),
                last_latency_ms: rec.and_then(|r| r.last_latency_ms),
                last_error: rec.and_then(|r| r.last_error.clone()),
            }
        })
        .collect())
}

/// 后台周期探测入口（setup 的延迟 30s + 每 6h 任务调用）。
pub async fn probe_all_from_app(app: &tauri::AppHandle) -> Result<Vec<HealthProbeResult>, String> {
    let engine = app.state::<crate::rules::RuleEngineState>().0.clone();
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("应用数据目录不可用: {e}"))?;
    let rules_dir = update::resolve_rules_dir(app);
    Ok(probe_all(&engine, &app_data, &rules_dir, None).await)
}

// ── 工具 ──────────────────────────────────────────────────────────────────

fn describe_exec_error(e: &RuleExecError) -> String {
    match e {
        RuleExecError::RuleNotFound { message } => format!("规则不可用: {message}"),
        RuleExecError::Timeout => "执行超时".to_string(),
        RuleExecError::Cancelled => "任务已取消".to_string(),
        RuleExecError::ScriptError { message, .. } => format!("脚本异常: {message}"),
        RuleExecError::Network { message } => format!("网络错误: {message}"),
        RuleExecError::BadReturn { message } => format!("返回结构不合法: {message}"),
    }
}

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    //! 源健康检查测试（spec §6.1 测试 10~14）。

    use super::*;
    use crate::rules::engine::{RuleEngine, RuleInput};
    use crate::rules::schema::{ContentType, RuleOrigin};
    use std::sync::Arc;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_engine() -> RuleEngine {
        RuleEngine::new(reqwest::Client::builder().build().expect("client 构建失败"))
    }

    fn make_manifest(search: &str) -> crate::rules::schema::RuleManifest {
        crate::rules::schema::RuleManifest {
            name: "测试源".to_string(),
            version: "1.0.0".to_string(),
            content_type: ContentType::Anime,
            base_url: "https://example.com".to_string(),
            language: "zh-CN".to_string(),
            nsfw: false,
            author: None,
            search: search.to_string(),
            detail: "function detail(url) { return {}; }".to_string(),
            chapter: "function chapter(detailUrl) { return []; }".to_string(),
            parse: "function parse(chapterUrl) { return { urls: [], kind: 'video' }; }".to_string(),
        }
    }

    /// 往临时规则目录写一条规则文件（含 probeKeyword，供 discover_rules 扫描）。
    fn write_rule(dir: &std::path::Path, subdir: &str, name: &str, search: &str, probe: &str) {
        let sub = dir.join(subdir);
        std::fs::create_dir_all(&sub).unwrap();
        let json = serde_json::json!({
            "name": name,
            "version": "1.0.0",
            "contentType": "anime",
            "baseUrl": "https://example.com",
            "language": "zh-CN",
            "nsfw": false,
            "probeKeyword": probe,
            "search": search,
            "detail": "function detail(url) { return {}; }",
            "chapter": "function chapter(detailUrl) { return []; }",
            "parse": "function parse(chapterUrl) { return { urls: [], kind: 'video' }; }"
        });
        std::fs::write(sub.join(name), serde_json::to_string_pretty(&json).unwrap()).unwrap();
    }

    // ── 测试 10：derive_status 全分支 ────────────────────────────────────────

    #[test]
    fn derive_status_covers_all_branches() {
        assert_eq!(derive_status(None), HealthStatus::Unknown);

        let mut rec = HealthRecord::new();
        assert_eq!(derive_status(Some(&rec)), HealthStatus::Healthy);
        rec.consecutive_failures = 1;
        assert_eq!(derive_status(Some(&rec)), HealthStatus::Degraded);
        rec.consecutive_failures = 2;
        assert_eq!(derive_status(Some(&rec)), HealthStatus::Degraded);
        rec.consecutive_failures = 3;
        assert_eq!(derive_status(Some(&rec)), HealthStatus::Abnormal);
        // 成功清零 → 回 Healthy
        rec.consecutive_failures = 0;
        assert_eq!(derive_status(Some(&rec)), HealthStatus::Healthy);
    }

    // ── 测试 11：探测成功但返回空数组 → 记为失败 ──────────────────────────────

    #[tokio::test]
    async fn probe_empty_result_counts_as_failure() {
        let engine = test_engine();
        let loaded = engine
            .load_rules(vec![RuleInput::Manifest {
                manifest: make_manifest("function search(k,p){ return []; }"),
                origin: RuleOrigin::Builtin,
            }])
            .await;
        assert_eq!(loaded[0].status, crate::rules::schema::RuleStatus::Ready);
        let id = loaded[0].id.clone();

        let result = probe_one(&engine, &id, "进击的巨人").await;
        assert!(!result.ok, "空数组应记为失败");
        assert!(result.error.as_deref().unwrap().contains("0 条"));
    }

    // ── 测试 12：单源超时 → 该源失败，其余源不受影响（并发隔离）───────────────

    #[tokio::test]
    async fn single_source_timeout_does_not_affect_others() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/slow"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_secs(30))
                    .set_body_string("late"),
            )
            .mount(&server)
            .await;
        let slow_url = format!("{}/slow", server.uri());

        let dir = tempfile::tempdir().unwrap();
        write_rule(
            dir.path(),
            "anime",
            "slow.json",
            &format!(
                "async function search(k,p) {{ await fetch('{slow_url}'); return [{{ title: k, url: 'https://example.com/x' }}]; }}"
            ),
            "慢关键词",
        );
        write_rule(
            dir.path(),
            "anime",
            "fast.json",
            "function search(k,p){ return [{ title: k, url: 'https://example.com/' + k }]; }",
            "快关键词",
        );

        let engine = test_engine();
        let loaded = engine
            .load_rules(vec![
                RuleInput::File {
                    path: dir.path().join("anime/slow.json"),
                    origin: RuleOrigin::Builtin,
                },
                RuleInput::File {
                    path: dir.path().join("anime/fast.json"),
                    origin: RuleOrigin::Builtin,
                },
            ])
            .await;
        assert!(
            loaded
                .iter()
                .all(|r| r.status == crate::rules::schema::RuleStatus::Ready),
            "两条规则都应加载成功: {:?}",
            loaded.iter().map(|r| &r.error).collect::<Vec<_>>()
        );

        let app_data = tempfile::tempdir().unwrap();
        let results = probe_all_with_timeout(
            &engine,
            app_data.path(),
            dir.path(),
            None,
            Duration::from_millis(400),
        )
        .await;

        let slow = results
            .iter()
            .find(|r| r.source_id == "slow")
            .expect("应有 slow 结果");
        assert!(!slow.ok, "慢源应在短超时内记为失败");
        assert!(slow.error.as_deref().unwrap().contains("超时"));
        let fast = results
            .iter()
            .find(|r| r.source_id == "fast")
            .expect("应有 fast 结果");
        assert!(fast.ok, "快源不应受慢源超时影响");

        // 落盘的健康记录同步更新
        let saved = load_health(&app_data.path().join("rules-health.json"));
        assert_eq!(saved["slow"].consecutive_failures, 1);
        assert_eq!(saved["fast"].consecutive_failures, 0);
    }

    // ── 测试 13：结果队列只保留最近 10 条；损坏文件降级为空记录 ────────────────

    #[test]
    fn health_record_keeps_last_ten_results() {
        let mut rec = HealthRecord::new();
        for i in 0..12i64 {
            let ok = i % 2 == 0;
            record_append(
                &mut rec,
                &HealthProbeResult {
                    source_id: "s".into(),
                    ok,
                    latency_ms: i as u64,
                    error: None,
                },
                i,
            );
        }
        assert_eq!(rec.results.len(), 10, "结果队列最多保留 10 条");
        assert_eq!(rec.results.front().unwrap().checked_at, 2, "旧记录被弹出");
        assert_eq!(rec.results.back().unwrap().checked_at, 11);
        // 12 条中 6 条成功（i 偶）6 条失败（i 奇），末尾为失败 → consecutive_failures=1
        assert_eq!(rec.consecutive_failures, 1);
    }

    #[test]
    fn corrupted_health_file_degrades_to_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rules-health.json");
        std::fs::write(&path, "not json {").unwrap();
        let map = load_health(&path);
        assert!(map.is_empty(), "损坏的健康文件应降级为空记录而非 panic");
    }

    #[test]
    fn save_and_load_health_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rules-health.json");
        let mut map = HashMap::new();
        let mut rec = HealthRecord::new();
        rec.consecutive_failures = 3;
        rec.last_error = Some("网络错误".into());
        map.insert("s1".to_string(), rec);
        save_health(&path, &map).unwrap();
        let loaded = load_health(&path);
        assert_eq!(loaded["s1"].consecutive_failures, 3);
        assert_eq!(loaded["s1"].last_error.as_deref(), Some("网络错误"));
    }

    // ── 测试 14：并发上限——探测 20 源时在飞请求 ≤8 ───────────────────────────

    #[tokio::test]
    async fn probe_concurrency_is_bounded_at_eight() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let in_flight = Arc::new(AtomicUsize::new(0));
        let max_seen = Arc::new(AtomicUsize::new(0));
        let targets: Vec<DiscoveredRule> = (0..20)
            .map(|i| DiscoveredRule {
                id: format!("s{i}"),
                keyword: "kw".into(),
            })
            .collect();

        let results = run_probes(targets, MAX_CONCURRENT_PROBES, {
            let in_flight = in_flight.clone();
            let max_seen = max_seen.clone();
            move |_t| {
                let in_flight = in_flight.clone();
                let max_seen = max_seen.clone();
                async move {
                    let cur = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                    max_seen.fetch_max(cur, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(30)).await;
                    in_flight.fetch_sub(1, Ordering::SeqCst);
                    HealthProbeResult {
                        source_id: "x".into(),
                        ok: true,
                        latency_ms: 1,
                        error: None,
                    }
                }
            }
        })
        .await;

        assert_eq!(results.len(), 20);
        let max = max_seen.load(Ordering::SeqCst);
        assert!(max <= MAX_CONCURRENT_PROBES, "并发上限应 ≤8，实际 {max}");
        assert!(max >= 5, "应体现并发（非串行），实际 {max}");
    }
}
