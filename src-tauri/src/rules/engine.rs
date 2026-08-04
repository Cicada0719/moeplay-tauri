//! 规则执行引擎：注册表 + 专用 worker 线程池 + CancellationToken 竞态取消。
//!
//! - 加载：`load_rules` 并行加载一批规则，每条独立 10s 编译超时，单条失败不影响其他。
//! - 执行：`search/detail/chapters/parse` 在 4 个常驻 worker 线程（一线程一沙箱）上
//!   运行，默认 15s 超时；`CancellationToken` 触发后中断对应 worker 的 JS 执行。
//! - 竞态：`new_scope_token`/`cancel_scope` 支持 FR-02 的 scope 级取消语义。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::rules::sandbox::{Sandbox, EXEC_TIMEOUT, LOAD_TIMEOUT};
use crate::rules::schema::{
    validate_manifest, ContentType, LoadedRule, RuleFileFormat, RuleLoadError, RuleManifest,
    RuleOrigin, RuleStatus,
};

/// 常驻 worker 线程数（QuickJS runtime 不可跨线程，必须一线程一沙箱）。
const WORKER_COUNT: usize = 4;

/// 搜索结果条目
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchItem {
    pub title: String,
    /// 详情页 URL（相对 base_url 亦可，前端会归一化）
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

/// 详情
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Detail {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

/// 章节（index 为 1-based，源切换定位依赖此字段）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chapter {
    pub id: String,
    pub title: String,
    pub url: String,
    pub index: u32,
}

/// 解析结果（播放/图片/文本资源地址）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseResult {
    pub urls: Vec<String>,
    /// "video" | "images" | "text"
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

/// 规则执行错误（`kind` tag 稳定，供前端结构化透传）
#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RuleExecError {
    #[error("规则不存在或无效: {0}")]
    RuleNotFound(String),
    #[error("规则执行超时")]
    Timeout,
    #[error("任务已取消")]
    Cancelled,
    #[error("脚本异常: {message}")]
    ScriptError {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        line: Option<u32>,
    },
    #[error("网络错误: {0}")]
    Network(String),
    #[error("返回结构不合法: {0}")]
    BadReturn(String),
}

/// 规则加载输入
#[derive(Debug, Clone)]
pub enum RuleInput {
    Manifest {
        manifest: RuleManifest,
        origin: RuleOrigin,
    },
    /// .json/.yaml/.yml 按扩展名解析
    File {
        path: PathBuf,
        origin: RuleOrigin,
    },
}

/// 派发给 worker 的任务
struct TaskMsg {
    rule_id: String,
    script: String,
    fn_name: String,
    args: Vec<serde_json::Value>,
    reply: tokio::sync::oneshot::Sender<Result<serde_json::Value, RuleExecError>>,
}

/// worker 句柄（引擎持有，用于分发任务与中断 JS）
struct WorkerHandle {
    tx: std::sync::mpsc::Sender<TaskMsg>,
    interrupt: Arc<AtomicBool>,
}

/// 规则引擎
pub struct RuleEngine {
    http: reqwest::Client,
    /// 仅存放 status=Ready 的规则（用于执行）
    rules: Arc<RwLock<HashMap<String, LoadedRule>>>,
    /// scope → CancellationToken
    scopes: Arc<Mutex<HashMap<String, CancellationToken>>>,
    workers: Vec<WorkerHandle>,
    next_worker: AtomicUsize,
}

fn spawn_worker(http: reqwest::Client) -> WorkerHandle {
    let (tx, rx) = std::sync::mpsc::channel::<TaskMsg>();
    let interrupt = Arc::new(AtomicBool::new(false));
    let handle = WorkerHandle {
        tx,
        interrupt: interrupt.clone(),
    };
    std::thread::spawn(move || {
        let sandbox = match Sandbox::new(http, interrupt.clone()) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("规则沙箱初始化失败: {e}");
                return;
            }
        };
        while let Ok(task) = rx.recv() {
            sandbox.reset_interrupt();
            sandbox.set_rule_id(&task.rule_id);
            let result = sandbox.call(&task.script, &task.fn_name, task.args);
            let _ = task.reply.send(result);
        }
    });
    handle
}

impl RuleEngine {
    /// 创建引擎（会立刻派生 4 个常驻 worker 线程）。
    pub fn new(http: reqwest::Client) -> Self {
        let workers = (0..WORKER_COUNT)
            .map(|_| spawn_worker(http.clone()))
            .collect();
        Self {
            http,
            rules: Arc::new(RwLock::new(HashMap::new())),
            scopes: Arc::new(Mutex::new(HashMap::new())),
            workers,
            next_worker: AtomicUsize::new(0),
        }
    }

    /// 并行加载一批规则；每条独立计时 10s，超时/失败仅影响该条。
    pub async fn load_rules(&self, inputs: Vec<RuleInput>) -> Vec<LoadedRule> {
        let futs = inputs.into_iter().map(|input| self.load_one(input));
        futures_util::future::join_all(futs).await
    }

    async fn load_one(&self, input: RuleInput) -> LoadedRule {
        let id = Uuid::new_v4().to_string();
        let (manifest, origin) = match input {
            RuleInput::Manifest { manifest, origin } => (manifest, origin),
            RuleInput::File { path, origin } => {
                let format = match RuleFileFormat::from_path(&path) {
                    Some(f) => f,
                    None => {
                        let err = RuleLoadError::schema("不支持的文件格式（仅 .json/.yaml/.yml）");
                        return LoadedRule::invalid(id, placeholder_manifest(), origin, err);
                    }
                };
                let text = match std::fs::read_to_string(&path) {
                    Ok(t) => t,
                    Err(e) => {
                        let err = RuleLoadError::schema(format!("读取规则文件失败: {e}"));
                        return LoadedRule::invalid(id, placeholder_manifest(), origin, err);
                    }
                };
                match RuleManifest::from_str(&text, format) {
                    Ok(m) => (m, origin),
                    Err(e) => return LoadedRule::invalid(id, placeholder_manifest(), origin, e),
                }
            }
        };

        if let Err(e) = validate_manifest(&manifest) {
            return LoadedRule::invalid(id, manifest, origin, e);
        }

        match self.compile_manifest(&manifest).await {
            Ok(()) => {
                let loaded = LoadedRule {
                    id: id.clone(),
                    manifest,
                    origin,
                    status: RuleStatus::Ready,
                    error: None,
                };
                self.rules.write().unwrap().insert(id, loaded.clone());
                loaded
            }
            Err(e) => LoadedRule::invalid(id, manifest, origin, e),
        }
    }

    /// 校验单个 manifest（schema 完整性 + JS 语法预编译），不执行。
    pub fn validate(manifest: &RuleManifest) -> Result<(), RuleLoadError> {
        validate_manifest(manifest)
    }

    /// 编译 4 个生命周期脚本（独立线程 + 10s 超时，死循环可被中断）。
    pub async fn compile_manifest(&self, manifest: &RuleManifest) -> Result<(), RuleLoadError> {
        let scripts: Vec<(String, String)> = vec![
            ("search".to_string(), manifest.search.clone()),
            ("detail".to_string(), manifest.detail.clone()),
            ("chapter".to_string(), manifest.chapter.clone()),
            ("parse".to_string(), manifest.parse.clone()),
        ];
        let http = self.http.clone();
        let interrupt = Arc::new(AtomicBool::new(false));
        let intr = interrupt.clone();
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), RuleLoadError>>();
        std::thread::spawn(move || {
            let sandbox = match Sandbox::new(http, intr) {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx.send(Err(RuleLoadError::compile(e, None)));
                    return;
                }
            };
            let mut outcome = Ok(());
            for (name, script) in &scripts {
                if let Err(e) = sandbox.compile_check(script, name) {
                    outcome = Err(e);
                    break;
                }
            }
            let _ = tx.send(outcome);
        });
        match tokio::time::timeout(LOAD_TIMEOUT, rx).await {
            Ok(Ok(res)) => res,
            Ok(Err(_)) => Err(RuleLoadError::timeout("规则编译通道关闭")),
            Err(_elapsed) => {
                interrupt.store(true, Ordering::Relaxed);
                Err(RuleLoadError::timeout(format!(
                    "规则编译超时（>{:?}）",
                    LOAD_TIMEOUT
                )))
            }
        }
    }

    /// 搜索
    pub async fn search(
        &self,
        rule_id: &str,
        keyword: &str,
        page: u32,
        token: CancellationToken,
    ) -> Result<Vec<SearchItem>, RuleExecError> {
        let manifest = self.get_manifest(rule_id)?;
        let value = self
            .execute(rule_id, manifest.search, "search", vec![json!(keyword), json!(page)], token)
            .await?;
        serde_json::from_value(value)
            .map_err(|e| RuleExecError::BadReturn(format!("搜索返回结构不合法: {e}")))
    }

    /// 详情
    pub async fn detail(
        &self,
        rule_id: &str,
        url: &str,
        token: CancellationToken,
    ) -> Result<Detail, RuleExecError> {
        let manifest = self.get_manifest(rule_id)?;
        let value = self
            .execute(rule_id, manifest.detail, "detail", vec![json!(url)], token)
            .await?;
        serde_json::from_value(value)
            .map_err(|e| RuleExecError::BadReturn(format!("详情返回结构不合法: {e}")))
    }

    /// 章节列表
    pub async fn chapters(
        &self,
        rule_id: &str,
        detail_url: &str,
        token: CancellationToken,
    ) -> Result<Vec<Chapter>, RuleExecError> {
        let manifest = self.get_manifest(rule_id)?;
        let value = self
            .execute(rule_id, manifest.chapter, "chapter", vec![json!(detail_url)], token)
            .await?;
        serde_json::from_value(value)
            .map_err(|e| RuleExecError::BadReturn(format!("章节返回结构不合法: {e}")))
    }

    /// 解析播放地址
    pub async fn parse(
        &self,
        rule_id: &str,
        chapter_url: &str,
        token: CancellationToken,
    ) -> Result<ParseResult, RuleExecError> {
        let manifest = self.get_manifest(rule_id)?;
        let value = self
            .execute(rule_id, manifest.parse, "parse", vec![json!(chapter_url)], token)
            .await?;
        serde_json::from_value(value)
            .map_err(|e| RuleExecError::BadReturn(format!("解析返回结构不合法: {e}")))
    }

    /// 取消指定 scope（如 "play:{contentId}"）下所有未完成任务。
    pub fn cancel_scope(&self, scope: &str) {
        let scopes = self.scopes.lock().unwrap();
        if let Some(token) = scopes.get(scope) {
            token.cancel();
        }
    }

    /// 为 scope 生成新 token（自动取消该 scope 旧 token）。
    pub fn new_scope_token(&self, scope: &str) -> CancellationToken {
        let token = CancellationToken::new();
        let mut scopes = self.scopes.lock().unwrap();
        if let Some(old) = scopes.insert(scope.to_string(), token.clone()) {
            old.cancel();
        }
        token
    }

    /// 注册一条已加载规则（用于导入链路）。
    pub fn register_loaded(&self, rule: LoadedRule) {
        if rule.status == RuleStatus::Ready {
            self.rules.write().unwrap().insert(rule.id.clone(), rule);
        }
    }

    /// 删除规则：仅允许 Custom；内置规则拒绝。
    pub fn remove_rule(&self, rule_id: &str) -> Result<(), String> {
        let mut rules = self.rules.write().unwrap();
        let rule = rules
            .get(rule_id)
            .ok_or_else(|| "规则不存在".to_string())?;
        if rule.origin != RuleOrigin::Custom {
            return Err("内置规则不可删除".to_string());
        }
        rules.remove(rule_id);
        Ok(())
    }

    /// 导出全部已加载规则的 manifest（内置 + 自定义）。
    pub fn all_manifests(&self) -> Vec<RuleManifest> {
        self.rules
            .read()
            .unwrap()
            .values()
            .map(|r| r.manifest.clone())
            .collect()
    }

    fn get_manifest(&self, rule_id: &str) -> Result<RuleManifest, RuleExecError> {
        self.rules
            .read()
            .unwrap()
            .get(rule_id)
            .map(|r| r.manifest.clone())
            .ok_or_else(|| RuleExecError::RuleNotFound(rule_id.to_string()))
    }

    /// 共享执行路径：分发到 worker → 监听 token 取消 / 15s 超时 → 中断 JS → 返回结果。
    async fn execute(
        &self,
        rule_id: &str,
        script: String,
        fn_name: &str,
        args: Vec<serde_json::Value>,
        token: CancellationToken,
    ) -> Result<serde_json::Value, RuleExecError> {
        let worker_idx = self.next_worker.fetch_add(1, Ordering::Relaxed) % self.workers.len();
        let worker = &self.workers[worker_idx];
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let task = TaskMsg {
            rule_id: rule_id.to_string(),
            script,
            fn_name: fn_name.to_string(),
            args,
            reply: reply_tx,
        };
        worker
            .tx
            .send(task)
            .map_err(|_| RuleExecError::Network("规则 worker 通道已关闭".into()))?;
        let interrupt = worker.interrupt.clone();

        tokio::select! {
            _ = token.cancelled() => {
                interrupt.store(true, Ordering::Relaxed);
                Err(RuleExecError::Cancelled)
            }
            res = tokio::time::timeout(EXEC_TIMEOUT, reply_rx) => {
                match res {
                    Ok(Ok(Ok(value))) => Ok(value),
                    Ok(Ok(Err(e))) => Err(e),
                    Ok(Err(_)) => Err(RuleExecError::Cancelled),
                    Err(_elapsed) => {
                        interrupt.store(true, Ordering::Relaxed);
                        Err(RuleExecError::Timeout)
                    }
                }
            }
        }
    }
}

fn placeholder_manifest() -> RuleManifest {
    RuleManifest {
        name: String::new(),
        version: String::new(),
        content_type: ContentType::Anime,
        base_url: String::new(),
        language: "zh-CN".to_string(),
        nsfw: false,
        author: None,
        search: String::new(),
        detail: String::new(),
        chapter: String::new(),
        parse: String::new(),
    }
}
