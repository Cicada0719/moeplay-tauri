//! 规则执行引擎：注册表 + 专用 worker 线程池 + CancellationToken 竞态取消。
//!
//! - 加载：`load_rules` 并行加载一批规则，每条独立 10s 编译超时，单条失败不影响其他。
//! - 执行：`search/detail/chapters/parse` 在 4 个常驻 worker 线程（一线程一沙箱）上
//!   运行，默认 15s 超时；`CancellationToken` 触发后中断对应 worker 的 JS 执行。
//! - 竞态：`new_scope_token`/`cancel_scope` 支持 FR-02 的 scope 级取消语义。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use futures_util::FutureExt;
use rquickjs::Runtime;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_util::sync::CancellationToken;

use crate::rules::sandbox::{Sandbox, EXEC_TIMEOUT, LOAD_TIMEOUT};
use crate::rules::schema::{
    file_stem_id, stable_rule_id, validate_manifest, ContentType, LoadedRule, RuleFileFormat,
    RuleLoadError, RuleManifest, RuleOrigin, RuleStatus,
};

/// 常驻 worker 线程数（QuickJS runtime 不可跨线程，必须一线程一沙箱）。
const WORKER_COUNT: usize = 4;

/// 取消/超时后等待 worker 真正退出当前任务的宽限期；超时则回收重建该 worker，
/// 避免其永久占用导致后续任务堆积（DeepSeek 审核项 1）。
const CANCEL_RECYCLE_TIMEOUT: Duration = Duration::from_secs(1);

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
    /// 任务所属取消 token：worker 取任务时若已取消则直接回 Cancelled，不执行，
    /// 从而避免「已取消任务先于取消信号入队」时 reset_interrupt 清掉中断标志的竞态。
    token: CancellationToken,
}

/// worker 句柄（引擎持有，用于分发任务与中断 JS）
struct WorkerHandle {
    tx: std::sync::mpsc::Sender<TaskMsg>,
    interrupt: Arc<AtomicBool>,
    /// worker 沙箱的 QuickJS Runtime 句柄（`parallel` feature 下 Send+Sync）。
    /// 取消/超时时引擎直接调用 `runtime.set_interrupt_handler` 向正在执行的 JS
    /// 发送中断（spec §3.3），而非只翻 AtomicBool 标志。
    runtime: Runtime,
}

/// 单 worker 槽位：句柄的读取（派发）与替换（回收）用**每槽独立 Mutex** 保护，
/// 而非全局池锁。任何临界区都不含 `.await`（只做 `tx`/`interrupt` 克隆或句柄替换），
/// 因此 `wait_worker_exit` 期间 `recycle_worker` 替换句柄不会与派发路径发生
/// 锁顺序死锁（DeepSeek 审核第 3 项）。
struct WorkerSlot {
    handle: Mutex<WorkerHandle>,
}

/// 规则引擎
pub struct RuleEngine {
    http: reqwest::Client,
    /// 全部已加载规则（Ready 与 Invalid 均登记；执行层只放行 Ready）。
    rules: Arc<RwLock<HashMap<String, LoadedRule>>>,
    /// scope → CancellationToken
    scopes: Arc<Mutex<HashMap<String, CancellationToken>>>,
    /// worker 槽位池（定长，取消/超时后仅替换槽内句柄；每槽独立锁，无全局池锁）。
    workers: Vec<WorkerSlot>,
    next_worker: AtomicUsize,
}

fn spawn_worker(http: reqwest::Client) -> WorkerHandle {
    let (tx, rx) = std::sync::mpsc::channel::<TaskMsg>();
    let interrupt = Arc::new(AtomicBool::new(false));
    // worker 线程持有的是 Arc 克隆；原句柄留在引擎侧用于派发/中断。
    let worker_interrupt = interrupt.clone();
    // 沙箱在 worker 线程内就地构建（QuickJS runtime 不可跨线程共享），但 Runtime 句柄
    // 需要回传给引擎用于取消/超时硬中断（`parallel` feature 下 Send+Sync）。
    let (rt_tx, rt_rx) = std::sync::mpsc::channel::<Runtime>();
    std::thread::spawn(move || {
        let sandbox = match Sandbox::new(http, worker_interrupt) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("规则沙箱初始化失败: {e}");
                return;
            }
        };
        let _ = rt_tx.send(sandbox.runtime());
        while let Ok(task) = rx.recv() {
            // 复位中断标记 + 重装 AtomicBool 中断处理器（清除引擎安装的硬中断残留）
            sandbox.rearm_interrupt();
            // 任务派发后、取到前已被取消：直接回 Cancelled，不执行（见 TaskMsg.token）。
            if task.token.is_cancelled() {
                let _ = task.reply.send(Err(RuleExecError::Cancelled));
                continue;
            }
            sandbox.set_rule_id(&task.rule_id);
            let result = sandbox.call(&task.script, &task.fn_name, task.args);
            let _ = task.reply.send(result);
        }
    });
    // 等 worker 线程把真实 Runtime 句柄传回来（仅一个，启动即就绪）。若沙箱初始化失败
    // 线程会直接退出、通道关闭——此时退化为「该 worker 不可用」，派发到它的任务会因
    // 通道关闭得到 Network 错误，与旧实现（沙箱失败→worker 静默空转）等价，不阻塞引擎。
    let runtime = match rt_rx.recv() {
        Ok(rt) => rt,
        Err(_) => {
            tracing::error!("规则 worker 沙箱初始化失败，该 worker 将不可用");
            Runtime::new().expect("failed to create fallback runtime")
        }
    };
    WorkerHandle {
        tx,
        interrupt,
        runtime,
    }
}

impl RuleEngine {
    /// 创建引擎（会立刻派生 4 个常驻 worker 线程）。
    pub fn new(http: reqwest::Client) -> Self {
        let workers = (0..WORKER_COUNT)
            .map(|_| WorkerSlot {
                handle: Mutex::new(spawn_worker(http.clone())),
            })
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
        let (id, manifest, origin) = match input {
            RuleInput::Manifest { manifest, origin } => {
                // 无文件名可用的输入 → 内容 hash 稳定 id（Kimi K3 复审第 2 项：重复
                // 加载同一 manifest 得同一 id，天然 upsert）。
                (stable_rule_id(&manifest), manifest, origin)
            }
            RuleInput::File { path, origin } => {
                // 文件规则 id = 文件名 stem（Kimi K3 复审第 1 项）：幂等且与磁盘文件
                // 一一对应，就地编辑内容也不变，删除链路 `custom_rules/{id}.json` 不断裂。
                let id = file_stem_id(&path);
                let format = match RuleFileFormat::from_path(&path) {
                    Some(f) => f,
                    None => {
                        let err = RuleLoadError::schema("不支持的文件格式（仅 .json/.yaml/.yml）");
                        return self.register_loaded(LoadedRule::invalid(
                            id,
                            placeholder_manifest(&path),
                            origin,
                            err,
                        ));
                    }
                };
                let text = match std::fs::read_to_string(&path) {
                    Ok(t) => t,
                    Err(e) => {
                        let err = RuleLoadError::schema(format!("读取规则文件失败: {e}"));
                        return self.register_loaded(LoadedRule::invalid(
                            id,
                            placeholder_manifest(&path),
                            origin,
                            err,
                        ));
                    }
                };
                match RuleManifest::from_str(&text, format) {
                    Ok(m) => (id, m, origin),
                    Err(e) => {
                        return self.register_loaded(LoadedRule::invalid(
                            id,
                            placeholder_manifest(&path),
                            origin,
                            e,
                        ))
                    }
                }
            }
        };

        if let Err(e) = validate_manifest(&manifest) {
            return self.register_loaded(LoadedRule::invalid(id, manifest, origin, e));
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
                self.register_loaded(loaded)
            }
            Err(e) => self.register_loaded(LoadedRule::invalid(id, manifest, origin, e)),
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

    /// 注册一条已加载规则（Ready 与 Invalid 均登记，供前端列表/置灰/删除引用）。
    pub fn register_loaded(&self, rule: LoadedRule) -> LoadedRule {
        self.rules
            .write()
            .unwrap()
            .insert(rule.id.clone(), rule.clone());
        rule
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

    /// 导出全部已加载规则的 manifest（内置 + 自定义，含 Invalid，供前端列表/导出展示）。
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
            // 执行层只放行 Ready：Invalid 规则虽已注册进 map（供列表/置灰），但不可执行。
            .filter(|r| r.status == RuleStatus::Ready)
            .map(|r| r.manifest.clone())
            .ok_or_else(|| RuleExecError::RuleNotFound(rule_id.to_string()))
    }

    /// 共享执行路径：分发到 worker → 监听 token 取消 / 15s 超时 → 中断 JS →
    /// 等待 worker 真正退出（宽限期内未退出则回收重建）→ 返回结果。
    async fn execute(
        &self,
        rule_id: &str,
        script: String,
        fn_name: &str,
        args: Vec<serde_json::Value>,
        token: CancellationToken,
    ) -> Result<serde_json::Value, RuleExecError> {
        let worker_idx = self.next_worker.fetch_add(1, Ordering::Relaxed);
        // 归一化到槽位：round-robin 计数器会超过槽数，回收路径必须使用**同一**槽位索引，
        // 否则会因 idx≥len 而错误跳过回收（曾导致卡死 worker 不被重建、后续任务堆积）。
        let slot_idx = worker_idx % self.workers.len();
        let (tx, interrupt, runtime) = {
            // 每槽独立锁，仅克隆 tx/interrupt/runtime（短临界区，无 await）。
            let guard = self.workers[slot_idx].handle.lock().unwrap();
            (guard.tx.clone(), guard.interrupt.clone(), guard.runtime.clone())
        };
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let task = TaskMsg {
            rule_id: rule_id.to_string(),
            script,
            fn_name: fn_name.to_string(),
            args,
            reply: reply_tx,
            token: token.clone(),
        };
        tx.send(task)
            .map_err(|_| RuleExecError::Network("规则 worker 通道已关闭".into()))?;

        // 结果 future：worker 真正完成当前任务后 resolve（成功/失败/通道关闭）。
        // Fuse 保证被 select 探测过后仍可安全二次 poll（等待 worker 退出）。
        let result_fut = async move {
            match reply_rx.await {
                Ok(Ok(v)) => Ok(v),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(RuleExecError::Cancelled), // worker 线程退出，通道关闭
            }
        }
        .fuse();
        tokio::pin!(result_fut);

        enum ExecOutcome {
            Value(Result<serde_json::Value, RuleExecError>),
            Cancelled,
            Timeout,
        }

        let outcome = tokio::select! {
            _ = token.cancelled() => ExecOutcome::Cancelled,
            r = &mut result_fut => ExecOutcome::Value(r),
            _ = tokio::time::sleep(EXEC_TIMEOUT) => ExecOutcome::Timeout,
        };

        match outcome {
            ExecOutcome::Value(r) => r,
            ExecOutcome::Cancelled => {
                // 竞态取消：置中断位中断 worker 的 JS，并等待其真正退出，避免后续任务堆积。
                interrupt.store(true, Ordering::Relaxed);
                // spec §3.3：真正调用 runtime.set_interrupt_handler 发 JS 中断（不只看标志位）。
                self.hard_interrupt_js(&runtime);
                self.wait_worker_exit(slot_idx, &mut result_fut).await;
                Err(RuleExecError::Cancelled)
            }
            ExecOutcome::Timeout => {
                interrupt.store(true, Ordering::Relaxed);
                // spec §3.3：同上，超时同样安装硬中断处理器兜底。
                self.hard_interrupt_js(&runtime);
                self.wait_worker_exit(slot_idx, &mut result_fut).await;
                Err(RuleExecError::Timeout)
            }
        }
    }

    /// 真正调用 `runtime.set_interrupt_handler` 向 worker 发送 JS 中断（spec §3.3）。
    ///
    /// 实现说明：`Context::with` 在整段 JS 执行期间持有 runtime 内部锁；若 worker 正被
    /// `await fetch(...)`（Rust `block_on`）阻塞，锁会被长时间占用，同步调用会阻塞当前
    /// async 取消路径。因此放进 `tokio::task::spawn_blocking` 异步执行：锁空闲时立即
    /// 生效，被占用时等锁释放后生效（AtomicBool 中断已先行触发 JS 中止并释放锁）。
    /// worker 每轮取任务前的 `Sandbox::rearm_interrupt` 会把它换回 AtomicBool 处理器，
    /// 避免硬中断残留影响下一次执行。
    fn hard_interrupt_js(&self, runtime: &Runtime) {
        let hard_rt = runtime.clone();
        // drop 掉 JoinHandle 即「detach」：后台线程设置硬中断处理器，调用方不等待。
        drop(tokio::task::spawn_blocking(move || {
            hard_rt.set_interrupt_handler(Some(Box::new(|| true)));
        }));
    }

    /// 等待被中断的 worker 真正退出当前任务；宽限期内未退出则回收重建该 worker。
    ///
    /// 锁设计：等待期间**不持有**任何槽位锁（`result_fut` 只是 oneshot receiver），
    /// 超时后才短时持有 `slot_idx` 槽位的独立锁做句柄替换，与派发路径互不阻塞。
    async fn wait_worker_exit(
        &self,
        slot_idx: usize,
        result_fut: &mut (impl std::future::Future<Output = Result<serde_json::Value, RuleExecError>> + Unpin),
    ) {
        if tokio::time::timeout(CANCEL_RECYCLE_TIMEOUT, result_fut)
            .await
            .is_err()
        {
            self.recycle_worker(slot_idx);
        }
    }

    /// 回收重建指定槽位的 worker：旧 worker 置中断位 + 通道关闭后自行退出，新 worker 立即接管。
    fn recycle_worker(&self, slot_idx: usize) {
        if slot_idx >= self.workers.len() {
            return;
        }
        let mut guard = self.workers[slot_idx].handle.lock().unwrap();
        guard.interrupt.store(true, Ordering::Relaxed);
        let fresh = spawn_worker(self.http.clone());
        tracing::warn!("规则 worker 中断后未在宽限期内退出，已回收重建 (slot={slot_idx})");
        *guard = fresh;
    }
}

/// 解析/读取失败时的占位 manifest：用文件 stem 作 name，避免列表/导出出现无名条目
/// （Kimi K3 复审非阻塞项）。
fn placeholder_manifest(path: &Path) -> RuleManifest {
    RuleManifest {
        name: file_stem_id(path),
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
