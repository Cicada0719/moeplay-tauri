//! QuickJS 沙箱：规则脚本在隔离环境中执行。
//!
//! 沙箱使用 `Context::full` 创建（注册 Eval/Promise/JSON 等标准内在函数），但
//! **不链接任何 `std`/`os` 模块**（QuickJS 的 `std`/`os` 属于独立 C 模块，
//! rquickjs 默认不引入，因此无文件系统/进程能力），仅注入两个全局：`fetch`
//! （走统一 reqwest 出口，带审计日志）与 `console.log/warn/error`（转发到
//! `tracing`）。单条规则脚本异常只影响其自身调用，不影响应用主体。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rquickjs::function::Opt;
use rquickjs::{Array, Context, Ctx, Exception, Function, Object, Promise, Runtime, Value};
use serde_json::{Map as JsonMap, Value as JsonValue};

use crate::rules::engine::RuleExecError;
use crate::rules::schema::RuleLoadError;

/// 单次生命周期执行默认超时（秒）
pub const EXEC_TIMEOUT: Duration = Duration::from_secs(15);
/// 单条规则加载（编译）超时（秒）
pub const LOAD_TIMEOUT: Duration = Duration::from_secs(10);
/// 沙箱内存上限
const MEMORY_LIMIT: usize = 64 * 1024 * 1024;
/// 沙箱最大栈
const MAX_STACK: usize = 1024 * 1024;
/// 注入 fetch 的单次网络超时
const FETCH_TIMEOUT: Duration = Duration::from_secs(30);

/// `fetch` 桥接的响应结构
struct FetchResponse {
    status: u16,
    body: String,
    headers: HashMap<String, String>,
}

/// 网络桥接器：在 worker 线程内使用独立的 tokio 当前线程 runtime 阻塞执行 reqwest。
struct FetchBridge {
    http: reqwest::Client,
    rule_id: Arc<Mutex<String>>,
}

impl FetchBridge {
    fn fetch(
        &self,
        url: &str,
        method: &str,
        headers: Vec<(String, String)>,
        body: Option<String>,
    ) -> Result<FetchResponse, String> {
        // worker 是非 async 线程，这里临时构建一个 current-thread runtime 阻塞等待。
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        rt.block_on(async {
            let method =
                reqwest::Method::from_bytes(method.as_bytes()).map_err(|e| e.to_string())?;
            let mut req = self.http.request(method, url);
            for (k, v) in &headers {
                req = req.header(k, v);
            }
            if let Some(b) = body {
                req = req.body(b);
            }
            let rule_id = self.rule_id.lock().map(|g| g.clone()).unwrap_or_default();
            let started = std::time::Instant::now();
            let resp = req
                .timeout(FETCH_TIMEOUT)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            let status = resp.status().as_u16();
            let resp_headers: HashMap<String, String> = resp
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();
            let body = resp.text().await.map_err(|e| e.to_string())?;
            tracing::info!(
                rule_id,
                url,
                status,
                elapsed_ms = started.elapsed().as_millis() as u64,
                "规则 fetch 审计"
            );
            Ok(FetchResponse {
                status,
                body,
                headers: resp_headers,
            })
        })
    }
}

/// QuickJS 沙箱封装。**一线程一实例，不可跨线程共享**（Runtime 带内部锁）。
pub struct Sandbox {
    // 保持 Runtime 句柄存活：虽然代码不直接读取该字段，但它的 Drop 决定
    // 底层 QuickJS 运行时何时释放，过早丢弃会使 Context 失效。
    #[allow(dead_code)]
    runtime: Runtime,
    context: Context,
    interrupt: Arc<AtomicBool>,
    rule_id: Arc<Mutex<String>>,
}

impl Sandbox {
    /// 创建沙箱：配置内存/栈上限，注册中断处理器，注入全局 `fetch`/`console`。
    pub fn new(http: reqwest::Client, interrupt: Arc<AtomicBool>) -> Result<Self, String> {
        let runtime = Runtime::new().map_err(|e| e.to_string())?;
        runtime.set_memory_limit(MEMORY_LIMIT);
        runtime.set_max_stack_size(MAX_STACK);
        let intr = interrupt.clone();
        runtime.set_interrupt_handler(Some(Box::new(move || intr.load(Ordering::Relaxed))));
        // `full` 注册标准内在函数（Eval/Promise/JSON/RegExp…），`base` 不含 Eval，
        // 会导致 `ctx.eval` 报 "eval is not supported"。`full` 不引入 std/os 模块，
        // 沙箱无文件系统/进程访问能力（见 `sandbox_no_fs_access` 测试）。
        let context = Context::full(&runtime).map_err(|e| e.to_string())?;
        let sandbox = Self {
            runtime,
            context,
            interrupt,
            rule_id: Arc::new(Mutex::new(String::new())),
        };
        sandbox.inject_globals(http).map_err(|e| e.to_string())?;
        Ok(sandbox)
    }

    /// 更新审计日志使用的 rule_id（worker 在每次任务前调用）。
    pub fn set_rule_id(&self, id: &str) {
        if let Ok(mut g) = self.rule_id.lock() {
            *g = id.to_string();
        }
    }

    /// 复位中断标记（worker 每次取任务后调用，避免残留中断影响下一次执行）。
    pub fn reset_interrupt(&self) {
        self.interrupt.store(false, Ordering::Relaxed);
    }

    fn inject_globals(&self, http: reqwest::Client) -> rquickjs::Result<()> {
        self.context.with(|ctx| {
            // ── console ──────────────────────────────────────────────────
            let console = Object::new(ctx.clone())?;
            let log = Function::new(ctx.clone(), |msg: Value| {
                let s: String = msg.get().unwrap_or_default();
                tracing::info!("[rule] console.log: {s}");
            })?;
            console.set("log", log)?;
            let warn = Function::new(ctx.clone(), |msg: Value| {
                let s: String = msg.get().unwrap_or_default();
                tracing::warn!("[rule] console.warn: {s}");
            })?;
            console.set("warn", warn)?;
            let error = Function::new(ctx.clone(), |msg: Value| {
                let s: String = msg.get().unwrap_or_default();
                tracing::error!("[rule] console.error: {s}");
            })?;
            console.set("error", error)?;
            ctx.globals().set("console", console)?;

            // ── fetch 桥接（统一 reqwest 出口 + 审计日志）────────────────
            let rule_id = self.rule_id.clone();
            let fetch_fn = build_fetch_function(ctx.clone(), http, rule_id)?;
            ctx.globals().set("fetch", fetch_fn)?;
            Ok(())
        })
    }

    /// 语法预编译：eval 顶层脚本（注册全局函数），捕获语法错误并提取行号。
    ///
    /// 顶层为函数声明的正常规则脚本不会被执行；顶层死循环等异常会在调用方
    /// （加载超时）中被中断，只影响该条规则。
    pub fn compile_check(&self, script: &str, _name: &str) -> Result<(), RuleLoadError> {
        self.context.with(|ctx| {
            ctx.eval::<Value, _>(script).map_err(|e| {
                let (message, line) = exception_info(&ctx, &e);
                RuleLoadError::compile(message, line)
            })?;
            Ok(())
        })
    }

    /// 执行一个生命周期函数：eval 脚本 → 取全局函数 → 传参调用 → 驱动 Promise → 转 JSON。
    pub fn call(
        &self,
        script: &str,
        fn_name: &str,
        args: Vec<JsonValue>,
    ) -> Result<JsonValue, RuleExecError> {
        self.context.with(|ctx| {
            // 1. 注册函数
            ctx.eval::<Value, _>(script).map_err(|e| {
                let (message, line) = exception_info(&ctx, &e);
                RuleExecError::ScriptError { message, line }
            })?;
            // 2. 取函数
            let globals = ctx.globals();
            let f: Function = globals.get(fn_name).map_err(|e| {
                RuleExecError::ScriptError {
                    message: format!("未找到函数 {fn_name}: {e}"),
                    line: None,
                }
            })?;
            // 3. 参数转换
            let js_args: Vec<Value> = args
                .iter()
                .map(|a| json_to_js(&ctx, a))
                .collect::<rquickjs::Result<_>>()
                .map_err(|e| RuleExecError::BadReturn(format!("参数转换失败: {e}")))?;
            // 4. 调用
            let result = call_dynamic(&ctx, &f, &js_args).map_err(|e| {
                let (message, line) = exception_info(&ctx, &e);
                RuleExecError::ScriptError { message, line }
            })?;
            // 5. 若返回 Promise 则驱动至完成
            if let Some(promise) = result.as_promise() {
                return drive_promise(promise.clone());
            }
            // 6. 转 JSON
            js_to_json(&result)
                .map_err(|e| RuleExecError::BadReturn(format!("返回结构不合法: {e}")))
        })
    }
}

/// 构建注入沙箱的全局 `fetch` 函数。
///
/// 通过外部泛型函数显式绑定 `'js` 生命周期，避免闭包内 `Ctx` 与返回 `Value`
/// 的更高阶生命周期无法统一的问题。返回已 resolve 的 Promise，兼容
/// `await fetch()` 与 `.then()` 两种调用方式。
fn build_fetch_function<'js>(
    ctx: Ctx<'js>,
    http: reqwest::Client,
    rule_id: Arc<Mutex<String>>,
) -> rquickjs::Result<Function<'js>> {
    Function::new(
        ctx,
        move |ctx: Ctx<'js>, url: String, options: Opt<Object<'js>>| -> rquickjs::Result<Value<'js>> {
            let (method, headers, body) = parse_fetch_options(options)?;
            let bridge = FetchBridge {
                http: http.clone(),
                rule_id: rule_id.clone(),
            };
            match bridge.fetch(&url, &method, headers, body) {
                Ok(resp) => {
                    let obj = Object::new(ctx.clone())?;
                    obj.set("status", resp.status)?;
                    obj.set("body", resp.body)?;
                    let h = Object::new(ctx.clone())?;
                    for (k, v) in &resp.headers {
                        h.set(k.as_str(), v.as_str())?;
                    }
                    obj.set("headers", h)?;
                    let (promise, resolve, _reject) = Promise::new(&ctx)?;
                    resolve.call::<_, ()>((obj,))?;
                    Ok(promise.into_value())
                }
                Err(e) => Err(Exception::throw_message(&ctx, &e)),
            }
        },
    )
}

/// 调用生命周期函数（支持 0~2 个参数；超出返回参数数量错误）。
fn call_dynamic<'js>(
    ctx: &Ctx<'js>,
    f: &Function<'js>,
    args: &[Value<'js>],
) -> rquickjs::Result<Value<'js>> {
    match args.len() {
        0 => f.call(()),
        1 => f.call((args[0].clone(),)),
        2 => f.call((args[0].clone(), args[1].clone())),
        n => Err(Exception::throw_message(
            ctx,
            &format!("不支持的参数数量: {n}"),
        )),
    }
}

/// 驱动 Promise 至 settled，返回其 resolve 值（reject 时返回结构化错误）。
fn drive_promise<'js>(promise: Promise<'js>) -> Result<JsonValue, RuleExecError> {
    match promise.finish::<Value>() {
        Ok(v) => js_to_json(&v)
            .map_err(|e| RuleExecError::BadReturn(format!("Promise 返回值不合法: {e}"))),
        Err(e) => {
            let (message, line) = exception_info(promise.ctx(), &e);
            Err(RuleExecError::ScriptError { message, line })
        }
    }
}

/// 解析后的 fetch 选项：(method, headers, body)
type ParsedFetchOptions = (String, Vec<(String, String)>, Option<String>);

/// 解析 fetch 的 options：method / headers / body。
fn parse_fetch_options<'js>(
    options: Opt<Object<'js>>,
) -> rquickjs::Result<ParsedFetchOptions> {
    let mut method = "GET".to_string();
    let mut headers = Vec::new();
    let mut body = None;
    if let Opt(Some(opts)) = options {
        if let Ok(m) = opts.get::<_, String>("method") {
            method = m.to_uppercase();
        }
        if let Ok(h) = opts.get::<_, Object>("headers") {
            for entry in h.props::<String, Value>() {
                let (k, v) = entry?;
                let vs: String = v.get().unwrap_or_default();
                headers.push((k, vs));
            }
        }
        if let Ok(b) = opts.get::<_, String>("body") {
            body = Some(b);
        }
    }
    Ok((method, headers, body))
}

/// 从上下文中取出当前异常，解析为 (message, line)。
fn exception_info(ctx: &Ctx, fallback: &rquickjs::Error) -> (String, Option<u32>) {
    let caught = ctx.catch();
    if caught.is_null() || caught.is_undefined() {
        (format!("{fallback}"), None)
    } else {
        (exception_message(&caught), exception_line(&caught))
    }
}

/// 提取异常对象的 message。
fn exception_message(caught: &Value) -> String {
    if let Some(obj) = caught.as_object() {
        if let Ok(m) = obj.get::<_, String>("message") {
            return m;
        }
    }
    if let Ok(s) = caught.get::<String>() {
        return s;
    }
    caught.type_of().as_str().to_string()
}

/// 从异常对象提取行号：优先读 `lineNumber`/`line` 属性，其次解析 stack/message。
fn exception_line(caught: &Value) -> Option<u32> {
    if let Some(obj) = caught.as_object() {
        if let Ok(n) = obj.get::<_, u32>("lineNumber") {
            if n > 0 {
                return Some(n);
            }
        }
        if let Ok(n) = obj.get::<_, u32>("line") {
            if n > 0 {
                return Some(n);
            }
        }
    }
    let mut text = String::new();
    if let Some(obj) = caught.as_object() {
        if let Ok(m) = obj.get::<_, String>("message") {
            text.push_str(&m);
        }
        if let Ok(s) = obj.get::<_, String>("stack") {
            text.push('\n');
            text.push_str(&s);
        }
    } else if let Ok(s) = caught.get::<String>() {
        text = s;
    }
    // 形如 `:3:7` / `line 3` / `<eval>:3:`
    let patterns = [
        regex::Regex::new(r":(\d+):\d+").unwrap(),
        regex::Regex::new(r"(?i)line[ =](\d+)").unwrap(),
    ];
    for re in &patterns {
        if let Some(caps) = re.captures(&text) {
            if let Some(m) = caps.get(1) {
                if let Ok(n) = m.as_str().parse::<u32>() {
                    return Some(n);
                }
            }
        }
    }
    None
}

/// 将 JS 值转换为 serde_json（手动递归，避免依赖 rquickjs 的 serde 特性）。
fn js_to_json<'js>(value: &Value<'js>) -> rquickjs::Result<JsonValue> {
    if value.is_null() || value.is_undefined() {
        return Ok(JsonValue::Null);
    }
    if let Some(b) = value.as_bool() {
        return Ok(JsonValue::Bool(b));
    }
    if let Some(i) = value.as_int() {
        return Ok(JsonValue::from(i));
    }
    if let Some(f) = value.as_float() {
        // 整数值落回 i32，避免前端按浮点解析丢失精度
        if f.fract() == 0.0 && f >= i32::MIN as f64 && f <= i32::MAX as f64 {
            return Ok(JsonValue::from(f as i32));
        }
        return Ok(json_float(f));
    }
    if let Some(s) = value.as_string() {
        let s: String = s.get()?;
        return Ok(JsonValue::String(s));
    }
    if let Some(arr) = value.as_array() {
        let mut out = Vec::new();
        for item in arr.iter::<Value>() {
            let item = item?;
            out.push(js_to_json(&item)?);
        }
        return Ok(JsonValue::Array(out));
    }
    if let Some(obj) = value.as_object() {
        let mut map = JsonMap::new();
        for entry in obj.props::<String, Value>() {
            let (k, v) = entry?;
            map.insert(k, js_to_json(&v)?);
        }
        return Ok(JsonValue::Object(map));
    }
    Ok(JsonValue::Null)
}

/// 将 f64 映射为 JSON 值，绝不静默丢弃为 null：
/// - 有限值（含超大/超小，如 `1e100`）交由 serde_json 以 JSON number 表示（精度按 f64）；
/// - NaN/±Infinity 无法用 JSON number 表示（`Number::from_f64` 返回 None），
///   显式转为字符串（"NaN" / "inf" / "-inf"），避免数据丢失。
fn json_float(f: f64) -> JsonValue {
    if let Some(n) = serde_json::Number::from_f64(f) {
        return JsonValue::Number(n);
    }
    JsonValue::String(f.to_string())
}

/// 将 serde_json 值转换为 JS 值（手动递归）。
fn json_to_js<'js>(ctx: &Ctx<'js>, value: &JsonValue) -> rquickjs::Result<Value<'js>> {
    match value {
        JsonValue::Null => Ok(Value::new_null(ctx.clone())),
        JsonValue::Bool(b) => Ok(Value::new_bool(ctx.clone(), *b)),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
                    return Ok(Value::new_int(ctx.clone(), i as i32));
                }
            }
            if let Some(f) = n.as_f64() {
                return Ok(Value::new_float(ctx.clone(), f));
            }
            Ok(Value::new_null(ctx.clone()))
        }
        JsonValue::String(s) => {
            let js = rquickjs::String::from_str(ctx.clone(), s)?;
            Ok(js.into_value())
        }
        JsonValue::Array(items) => {
            let arr = Array::new(ctx.clone())?;
            for (i, item) in items.iter().enumerate() {
                let v = json_to_js(ctx, item)?;
                arr.set(i, v)?;
            }
            Ok(arr.into_value())
        }
        JsonValue::Object(map) => {
            let obj = Object::new(ctx.clone())?;
            for (k, v) in map {
                let val = json_to_js(ctx, v)?;
                obj.set(k.as_str(), val)?;
            }
            Ok(obj.into_value())
        }
    }
}
