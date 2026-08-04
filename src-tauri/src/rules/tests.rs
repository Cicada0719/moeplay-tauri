//! 规则引擎测试（对应 spec §6.1 测试 1~14）。

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::engine::{RuleEngine, RuleExecError, RuleInput};
use super::sandbox::Sandbox;
use super::schema::{ContentType, RuleManifest, RuleOrigin, RuleStatus};

fn test_http() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("moeplay/2.0")
        .build()
        .unwrap()
}

fn test_engine() -> RuleEngine {
    RuleEngine::new(test_http())
}

fn make_manifest(name: &str, search: &str) -> RuleManifest {
    RuleManifest {
        name: name.to_string(),
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

// ── 测试 4：语法错误行号 ────────────────────────────────────────────────

#[tokio::test]
async fn compile_syntax_error_has_line() {
    let bad_search = "function search(keyword, page) {\n  return [];\n  let x = ;\n}";
    let engine = test_engine();
    let loaded = engine
        .load_rules(vec![RuleInput::Manifest {
            manifest: make_manifest("语法错误源", bad_search),
            origin: RuleOrigin::Builtin,
        }])
        .await;
    let rule = &loaded[0];
    assert_eq!(rule.status, RuleStatus::Invalid);
    let error = rule.error.as_ref().expect("Invalid 规则应有 error");
    assert_eq!(error.phase, "compile");
    assert_eq!(error.line, Some(3));
}

// ── 测试 5：部分失败隔离 + 并行加载耗时 < 15s ────────────────────────────

#[tokio::test]
async fn load_rules_partial_failure() {
    let mut inputs = Vec::new();
    for i in 0..15 {
        inputs.push(RuleInput::Manifest {
            manifest: make_manifest(&format!("正常源{i}"), "function search(k,p){ return [{title:k,url:'https://example.com/'+k}]; }"),
            origin: RuleOrigin::Builtin,
        });
    }
    // 5 条顶层死循环脚本：含 function 关键字（通过 schema 校验），但 eval 挂起 → 10s 超时
    let bad = "function search(k,p){ return []; }\nwhile(true) {}";
    for i in 0..5 {
        inputs.push(RuleInput::Manifest {
            manifest: make_manifest(&format!("死循环源{i}"), bad),
            origin: RuleOrigin::Builtin,
        });
    }
    let engine = test_engine();
    let started = std::time::Instant::now();
    let loaded = engine.load_rules(inputs).await;
    let elapsed = started.elapsed();
    assert_eq!(loaded.len(), 20);
    let ready = loaded.iter().filter(|r| r.status == RuleStatus::Ready).count();
    let invalid = loaded.iter().filter(|r| r.status == RuleStatus::Invalid).count();
    assert_eq!(ready, 15, "15 条规则应可用");
    assert_eq!(invalid, 5, "5 条死循环规则应被标记无效");
    assert!(
        elapsed < Duration::from_secs(15),
        "并行加载总耗时应 < 15s，实际 {elapsed:?}"
    );
}

// ── 测试 6：执行超时 ────────────────────────────────────────────────────

#[tokio::test]
async fn exec_timeout() {
    let engine = test_engine();
    let loaded = engine
        .load_rules(vec![RuleInput::Manifest {
            manifest: make_manifest("超时源", "function search(k,p){ while(true){} }"),
            origin: RuleOrigin::Builtin,
        }])
        .await;
    assert_eq!(loaded[0].status, RuleStatus::Ready);
    let id = loaded[0].id.clone();
    let token = CancellationToken::new();
    let result = tokio::time::timeout(
        Duration::from_secs(16),
        engine.search(&id, "x", 1, token),
    )
    .await
    .expect("执行超时应 ≤16s 返回");
    assert!(matches!(result, Err(RuleExecError::Timeout)));
}

// ── 测试 7：脚本异常透传 ────────────────────────────────────────────────

#[tokio::test]
async fn exec_script_error_propagates() {
    let engine = test_engine();
    let loaded = engine
        .load_rules(vec![RuleInput::Manifest {
            manifest: make_manifest("抛错源", "function search(k,p){ throw new Error('boom'); }"),
            origin: RuleOrigin::Builtin,
        }])
        .await;
    assert_eq!(loaded[0].status, RuleStatus::Ready);
    let id = loaded[0].id.clone();
    let token = CancellationToken::new();
    let err = engine.search(&id, "x", 1, token).await.unwrap_err();
    match err {
        RuleExecError::ScriptError { message, .. } => {
            assert!(message.contains("boom"), "message: {message}");
        }
        other => panic!("期望 ScriptError，得到 {other:?}"),
    }
}

// ── 测试 8：取消 + 中断生效（worker 不被永久占用）────────────────────────

#[tokio::test]
async fn exec_cancelled() {
    let engine = test_engine();
    let loaded = engine
        .load_rules(vec![RuleInput::Manifest {
            manifest: make_manifest("取消源", "function search(k,p){ return [{title:k,url:'https://example.com/x'}]; }")
                .with_parse("function parse(chapterUrl) { while(true){} }"),
            origin: RuleOrigin::Builtin,
        }])
        .await;
    assert_eq!(loaded[0].status, RuleStatus::Ready);
    let id = loaded[0].id.clone();

    let token = engine.new_scope_token("play:x");
    let parse_fut = engine.parse(&id, "https://example.com/1", token.clone());
    tokio::time::sleep(Duration::from_millis(300)).await;
    engine.cancel_scope("play:x");
    let err = parse_fut.await.unwrap_err();
    assert!(matches!(err, RuleExecError::Cancelled));

    // worker 未被永久占用：轮询 4 次回到原 worker，全部应正常成功
    let all_ok = tokio::time::timeout(Duration::from_secs(8), async {
        for _ in 0..4 {
            let token = CancellationToken::new();
            let items = engine.search(&id, "x", 1, token).await.expect("worker 应被释放");
            assert_eq!(items.len(), 1);
        }
    })
    .await;
    assert!(all_ok.is_ok(), "被中断的 worker 应可继续执行后续任务");
}

// ── 测试 9：scope token 替换 ────────────────────────────────────────────

#[test]
fn scope_token_replacement() {
    let engine = test_engine();
    let first = engine.new_scope_token("play:x");
    let _second = engine.new_scope_token("play:x");
    assert!(first.is_cancelled(), "旧 token 应被自动取消");
}

// ── 测试 10：沙箱无文件系统 / 进程访问能力 ──────────────────────────────

#[test]
fn sandbox_no_fs_access() {
    let interrupt = Arc::new(AtomicBool::new(false));
    let sandbox = Sandbox::new(test_http(), interrupt).unwrap();
    let err = sandbox
        .call("function search(k,p){ return std.open('/etc/passwd'); }", "search", vec![])
        .unwrap_err();
    assert!(
        matches!(err, RuleExecError::ScriptError { .. }),
        "std 不应存在，应抛 ScriptError"
    );
    let err2 = sandbox
        .call("function search(k,p){ return os.system('echo hi'); }", "search", vec![])
        .unwrap_err();
    assert!(
        matches!(err2, RuleExecError::ScriptError { .. }),
        "os 不应存在，应抛 ScriptError"
    );
}

// ── 测试：沙箱全局暴露面最小化（WebView/JSContext 对象一概不暴露）─────────

#[test]
fn sandbox_global_surface_is_minimal() {
    let interrupt = Arc::new(AtomicBool::new(false));
    let sandbox = Sandbox::new(test_http(), interrupt).unwrap();
    let result = sandbox
        .call(
            r#"function search(k,p) {
                // WebView/JSContext / 宿主全局对象必须不存在；QuickJS 也未加载 std/os
                const dangerous = ['window','navigator','document','process','require','global','Deno','Buffer','std','os','__TAURI__'];
                const present = dangerous.filter(g => typeof globalThis[g] !== 'undefined');
                return {
                    present,
                    hasFetch: typeof fetch === 'function',
                    hasConsole: typeof console === 'object',
                    hasPromise: typeof Promise === 'function',
                    hasJson: typeof JSON === 'object',
                };
            }"#,
            "search",
            vec![],
        )
        .expect("脚本应正常执行");
    let obj = result.as_object().expect("应返回对象");
    assert_eq!(
        obj.get("present"),
        Some(&serde_json::json!([])),
        "不应暴露任何宿主/WebView 全局对象: {:?}",
        obj.get("present")
    );
    assert_eq!(obj.get("hasFetch"), Some(&serde_json::json!(true)));
    assert_eq!(obj.get("hasConsole"), Some(&serde_json::json!(true)));
    // Promise/JSON 是规则运行必需（async/await、结构化解析），显式注入
    assert_eq!(obj.get("hasPromise"), Some(&serde_json::json!(true)));
    assert_eq!(obj.get("hasJson"), Some(&serde_json::json!(true)));
}

// ── 测试：注入前已存在全局 fetch → 强制覆盖为 reqwest 桥接（DeepSeek 审核第 2 项）─

#[tokio::test]
async fn inject_fetch_overwrites_preexisting() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/bridge"))
        .respond_with(ResponseTemplate::new(200).set_body_string("bridge-body"))
        .mount(&server)
        .await;

    let url = format!("{}/bridge", server.uri());
    let interrupt = Arc::new(AtomicBool::new(false));
    // 沙箱内 fetch 走 FetchBridge::block_on（Rust current-thread runtime），不能在 tokio
    // 测试线程上直接执行（会触发「runtime 内建 runtime」）。沙箱 QuickJS 运行时不可跨
    // 线程，因此在独立 std 线程内**就地构建**沙箱并调用——与 worker 线程真实环境一致。
    let (tx, rx) = std::sync::mpsc::channel::<Result<serde_json::Value, String>>();
    std::thread::spawn(move || {
        let sandbox = Sandbox::new_with_preexisting_fetch(test_http(), interrupt).unwrap();
        let script = format!(
            "async function search(k,p) {{ const res = await fetch('{url}'); return {{ body: res.body }}; }}"
        );
        let result = sandbox.call(&script, "search", vec![]).map_err(|e| format!("{e:?}"));
        let _ = tx.send(result);
    });
    let result = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("沙箱调用应在超时内完成")
        .expect("桥接 fetch 应生效");
    let obj = result.as_object().expect("应返回对象");
    // 若伪造 fetch 未被覆盖，这里会是 "fake" 且服务端收不到请求
    assert_eq!(obj.get("body"), Some(&serde_json::json!("bridge-body")));
    let requests = server.received_requests().await.unwrap();
    assert!(
        requests.iter().any(|r| r.url.path() == "/bridge"),
        "网络请求必须经由 reqwest 桥接到达服务端"
    );
}

// ── 测试 11：fetch 桥接 + UA 审计 ───────────────────────────────────────

#[tokio::test]
async fn fetch_bridge_audit() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/hello"))
        .respond_with(ResponseTemplate::new(200).set_body_string("hello-world"))
        .mount(&server)
        .await;

    let engine = test_engine();
    let url = format!("{}/hello", server.uri());
    let script = format!(
        "async function search(k,p) {{ const res = await fetch('{url}'); return [{{ title: res.body, url: 'https://example.com/' + res.status }}]; }}"
    );
    let loaded = engine
        .load_rules(vec![RuleInput::Manifest {
            manifest: make_manifest("fetch源", &script),
            origin: RuleOrigin::Builtin,
        }])
        .await;
    assert_eq!(loaded[0].status, RuleStatus::Ready);
    let id = loaded[0].id.clone();
    let token = CancellationToken::new();
    let items = engine.search(&id, "x", 1, token).await.expect("fetch 应成功");
    assert_eq!(items[0].title, "hello-world");
    assert_eq!(items[0].url, "https://example.com/200");

    // 审计：UA 必须是 moeplay/2.0
    let requests = server.received_requests().await.unwrap();
    let ua = requests
        .iter()
        .find(|r| r.url.path() == "/hello")
        .and_then(|r| r.headers.get("user-agent"))
        .and_then(|v| v.to_str().ok());
    assert_eq!(ua, Some("moeplay/2.0"));
}

// ── 测试 12：返回结构不合法 ─────────────────────────────────────────────

#[tokio::test]
async fn bad_return_shape() {
    let engine = test_engine();
    let loaded = engine
        .load_rules(vec![RuleInput::Manifest {
            manifest: make_manifest("坏返回源", "function search(k,p){ return 'hello'; }"),
            origin: RuleOrigin::Builtin,
        }])
        .await;
    assert_eq!(loaded[0].status, RuleStatus::Ready);
    let id = loaded[0].id.clone();
    let token = CancellationToken::new();
    let err = engine.search(&id, "x", 1, token).await.unwrap_err();
    assert!(matches!(err, RuleExecError::BadReturn(_)));
}

// ── 测试 13：导入 / 导出 / 删除往返 ─────────────────────────────────────

#[tokio::test]
async fn import_export_roundtrip() {
    let engine = test_engine();
    let dir = tempfile::tempdir().unwrap();

    // 自定义规则文件
    let custom_path = dir.path().join("custom_rule.json");
    let manifest = make_manifest("自定义源", "function search(k,p){ return [{title:k,url:'https://example.com'}]; }");
    std::fs::write(&custom_path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();

    let loaded = engine
        .load_rules(vec![RuleInput::File {
            path: custom_path,
            origin: RuleOrigin::Custom,
        }])
        .await;
    assert_eq!(loaded[0].status, RuleStatus::Ready);
    assert_eq!(loaded[0].origin, RuleOrigin::Custom);
    let custom_id = loaded[0].id.clone();

    // 导出应包含自定义规则
    let manifests = engine.all_manifests();
    assert!(manifests.iter().any(|m| m.name == "自定义源"));

    // 删除自定义规则 → 列表移除
    engine.remove_rule(&custom_id).unwrap();
    let manifests = engine.all_manifests();
    assert!(!manifests.iter().any(|m| m.name == "自定义源"));

    // 内置规则拒绝删除
    let builtin = engine
        .load_rules(vec![RuleInput::Manifest {
            manifest: make_manifest("内置源", "function search(k,p){ return []; }"),
            origin: RuleOrigin::Builtin,
        }])
        .await;
    let builtin_id = builtin[0].id.clone();
    let err = engine.remove_rule(&builtin_id).unwrap_err();
    assert!(err.contains("内置规则不可删除"), "err: {err}");
}

// ── 测试 14：kazumi 兼容 fixture ────────────────────────────────────────

#[tokio::test]
async fn kazumi_compat_fixture() {
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/kazumi-rule.json"
    );
    let engine = test_engine();
    let loaded = engine
        .load_rules(vec![RuleInput::File {
            path: std::path::PathBuf::from(fixture),
            origin: RuleOrigin::Builtin,
        }])
        .await;
    assert_eq!(loaded.len(), 1);
    assert_eq!(
        loaded[0].status,
        RuleStatus::Ready,
        "kazumi 风格 fixture 应加载成功: {:?}",
        loaded[0].error
    );
}

// ── 测试：取消时 worker 被 fetch 阻塞 → 超时回收重建，后续任务不堆积 ─────────

#[tokio::test]
async fn exec_cancel_recycles_fetch_blocked_worker() {
    // 端点延迟 10s：让规则脚本的 `await fetch(...)` 长时间阻塞 worker 线程
    // （阻塞发生在 Rust block_on 内，QuickJS 中断 handler 不生效）。
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/slow"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(10))
                .set_body_string("late"),
        )
        .mount(&server)
        .await;

    let engine = test_engine();
    let slow_url = format!("{}/slow", server.uri());
    let slow_script = format!("async function search(k,p) {{ await fetch('{slow_url}'); return []; }}");
    let loaded = engine
        .load_rules(vec![RuleInput::Manifest {
            manifest: make_manifest("慢源", &slow_script),
            origin: RuleOrigin::Builtin,
        }])
        .await;
    assert_eq!(loaded[0].status, RuleStatus::Ready);
    let slow_id = loaded[0].id.clone();

    // 快速规则：回收后的 worker 应能立刻承接
    let fast = engine
        .load_rules(vec![RuleInput::Manifest {
            manifest: make_manifest(
                "快源",
                "function search(k,p){ return [{title:k,url:'https://example.com/x'}]; }",
            ),
            origin: RuleOrigin::Builtin,
        }])
        .await;
    let fast_id = fast[0].id.clone();

    // 首个派发落到 worker[0]，随后在 fetch 阻塞期间取消
    let token = engine.new_scope_token("recycle:x");
    let search_fut = engine.search(&slow_id, "x", 1, token.clone());
    tokio::time::sleep(Duration::from_millis(300)).await;
    engine.cancel_scope("recycle:x");

    let cancelled = tokio::time::timeout(Duration::from_secs(5), search_fut).await;
    assert!(
        matches!(cancelled, Ok(Err(RuleExecError::Cancelled))),
        "取消应快速返回 Cancelled（含回收宽限），实际 {cancelled:?}"
    );

    // worker[0] 应已被回收重建：4 次派发（第 4 次回落到 worker[0]）全部快速完成，
    // 不会堆积在旧 worker 的 fetch 阻塞上。
    let all_ok = tokio::time::timeout(Duration::from_secs(4), async {
        for _ in 0..4 {
            let token = CancellationToken::new();
            let items = engine
                .search(&fast_id, "x", 1, token)
                .await
                .expect("worker 应已释放");
            assert_eq!(items.len(), 1);
        }
    })
    .await;
    assert!(all_ok.is_ok(), "被回收的 worker 不应拖慢后续任务");
}

// ── 测试：回收路径按「槽位索引」归一化（round-robin 计数超过槽数后仍正确回收）──

#[tokio::test]
async fn recycle_worker_slot_index_normalized() {
    // 端点延迟 10s：让槽位 0 的 worker 阻塞在 Rust block_on（中断 handler 不生效）
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/slow"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(10))
                .set_body_string("late"),
        )
        .mount(&server)
        .await;

    let engine = test_engine();
    let slow_url = format!("{}/slow", server.uri());
    let slow_script = format!("async function search(k,p) {{ await fetch('{slow_url}'); return []; }}");
    let slow = engine
        .load_rules(vec![RuleInput::Manifest {
            manifest: make_manifest("慢源", &slow_script),
            origin: RuleOrigin::Builtin,
        }])
        .await;
    assert_eq!(slow[0].status, RuleStatus::Ready);
    let slow_id = slow[0].id.clone();

    let fast = engine
        .load_rules(vec![RuleInput::Manifest {
            manifest: make_manifest(
                "快源",
                "function search(k,p){ return [{title:k,url:'https://example.com/x'}]; }",
            ),
            origin: RuleOrigin::Builtin,
        }])
        .await;
    let fast_id = fast[0].id.clone();

    // 先把 round-robin 计数器推进到 ≥4（占用槽位 0..3），让下一次派发落到槽位 0
    // 时 worker_idx = 4。旧实现 recycle_worker(4) 因 4 ≥ len(4) 会静默跳过回收，
    // 导致槽位 0 卡死；新实现按 slot_idx = 4 % 4 = 0 归一化，正确回收。
    for _ in 0..4 {
        let token = CancellationToken::new();
        engine.search(&fast_id, "x", 1, token).await.unwrap();
    }

    let token = engine.new_scope_token("recycle:normalize");
    let search_fut = engine.search(&slow_id, "x", 1, token.clone());
    tokio::time::sleep(Duration::from_millis(300)).await;
    engine.cancel_scope("recycle:normalize");

    let cancelled = tokio::time::timeout(Duration::from_secs(5), search_fut).await;
    assert!(
        matches!(cancelled, Ok(Err(RuleExecError::Cancelled))),
        "取消应快速返回 Cancelled（含回收宽限），实际 {cancelled:?}"
    );

    // 4 次派发（第 4 次回落槽位 0）都应快速完成：证明槽位 0 已被重建而非仍卡在 fetch 上
    let all_ok = tokio::time::timeout(Duration::from_secs(4), async {
        for _ in 0..4 {
            let token = CancellationToken::new();
            let items = engine
                .search(&fast_id, "x", 1, token)
                .await
                .expect("worker 应已释放");
            assert_eq!(items.len(), 1);
        }
    })
    .await;
    assert!(all_ok.is_ok(), "回收后槽位 0 不应拖慢后续任务");
}

// ── 测试：超大/非有限 float 不静默丢弃为 null ─────────────────────────────

#[test]
fn js_to_json_non_finite_and_huge_float() {
    let interrupt = Arc::new(AtomicBool::new(false));
    let sandbox = Sandbox::new(test_http(), interrupt).unwrap();
    let result = sandbox
        .call(
            "function search(k,p){ return { nan: NaN, inf: Infinity, negInf: -Infinity, huge: 1e100 }; }",
            "search",
            vec![],
        )
        .expect("脚本应正常执行");
    let obj = result.as_object().expect("应返回对象");
    assert_eq!(obj.get("nan"), Some(&serde_json::json!("NaN")));
    assert_eq!(obj.get("inf"), Some(&serde_json::json!("inf")));
    assert_eq!(obj.get("negInf"), Some(&serde_json::json!("-inf")));
    // 超大有限浮点保留为 JSON number，绝不为 null
    assert_eq!(obj.get("huge"), Some(&serde_json::json!(1e100)));
}

// ── 测试：Invalid 规则也注册进内部 map（列表/置灰/删除依赖）────────────────

#[tokio::test]
async fn invalid_rules_are_registered_in_map() {
    let engine = test_engine();
    let loaded = engine
        .load_rules(vec![RuleInput::Manifest {
            manifest: make_manifest("坏语法源", "function search(k,p){ return []; }")
                .with_parse("function parse(chapterUrl) { let x = ; }"),
            origin: RuleOrigin::Custom,
        }])
        .await;
    assert_eq!(loaded[0].status, RuleStatus::Invalid);
    let id = loaded[0].id.clone();

    // Invalid 规则已注册 → all_manifests 可见（供前端置灰/导出）
    let manifests = engine.all_manifests();
    assert!(
        manifests.iter().any(|m| m.name == "坏语法源"),
        "Invalid 规则应出现在 all_manifests"
    );

    // 执行层保持「仅 Ready 可执行」不变量
    let token = CancellationToken::new();
    let err = engine.search(&id, "x", 1, token).await.unwrap_err();
    assert!(matches!(err, RuleExecError::RuleNotFound(_)));

    // 自定义 Invalid 规则可按 id 删除
    engine.remove_rule(&id).unwrap();
    let manifests = engine.all_manifests();
    assert!(!manifests.iter().any(|m| m.name == "坏语法源"));
}

// ── 辅助扩展 ────────────────────────────────────────────────────────────

impl RuleManifest {
    fn with_parse(mut self, parse: &str) -> Self {
        self.parse = parse.to_string();
        self
    }
}
