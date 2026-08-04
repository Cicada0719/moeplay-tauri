//! 规则引擎测试（对应 spec §6.1 测试 1~14）。

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::engine::{RuleEngine, RuleExecError, RuleInput};
use super::sandbox::Sandbox;
use super::schema::{
    ContentType, LoadedRule, RuleFileFormat, RuleManifest, RuleOrigin, RuleStatus,
};
use crate::commands::{import_rule_to_dir, unique_custom_rule_id};

fn test_http() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("moeplay/", env!("CARGO_PKG_VERSION")))
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
            manifest: make_manifest(
                &format!("正常源{i}"),
                "function search(k,p){ return [{title:k,url:'https://example.com/'+k}]; }",
            ),
            origin: RuleOrigin::Builtin,
        });
    }
    // 5 条顶层语法错误脚本：含 function 关键字（通过 schema 校验），compile_check 是
    // 纯语法检查（不执行顶层代码），在编译期即被标记 Invalid，不占用 10s 加载超时。
    let bad = "function search(k,p){ return []; }\nlet x = ;";
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
    let ready = loaded
        .iter()
        .filter(|r| r.status == RuleStatus::Ready)
        .count();
    let invalid = loaded
        .iter()
        .filter(|r| r.status == RuleStatus::Invalid)
        .count();
    assert_eq!(ready, 15, "15 条规则应可用");
    assert_eq!(invalid, 5, "5 条死循环规则应被标记无效");
    assert!(
        elapsed < Duration::from_secs(15),
        "并行加载总耗时应 < 15s，实际 {elapsed:?}"
    );
}

// ── 测试 4b：compile_check 纯语法检查，顶层死循环只编译不执行（DeepSeek 复审第 4 项）─

#[tokio::test]
async fn compile_check_does_not_execute_toplevel_deadloop() {
    let engine = test_engine();
    let started = std::time::Instant::now();
    let loaded = engine
        .load_rules(vec![RuleInput::Manifest {
            manifest: make_manifest(
                "顶层死循环源",
                "function search(k,p){ return []; }\nwhile(true) {}",
            ),
            origin: RuleOrigin::Builtin,
        }])
        .await;
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "纯语法检查应立即返回，不应被顶层死循环卡住（实际 {:?}）",
        started.elapsed()
    );
    // 顶层死循环语法合法 → 规则标记为 Ready；只有运行时（call 会 eval 顶层代码）才会
    // 真正执行它，被引擎的取消/超时拦截，而不是在编译期就被误伤。
    assert_eq!(loaded[0].status, RuleStatus::Ready);
    let id = loaded[0].id.clone();

    // 运行时执行：验证顶层死循环会被取消拦截（worker 不被永久占用），且是运行期
    // 行为而非编译期错误。
    let token = engine.new_scope_token("deadloop:x");
    let search_fut = engine.search(&id, "x", 1, token.clone());
    tokio::time::sleep(Duration::from_millis(50)).await;
    engine.cancel_scope("deadloop:x");
    let err = tokio::time::timeout(Duration::from_secs(5), search_fut)
        .await
        .expect("取消应快速返回")
        .unwrap_err();
    assert!(matches!(err, RuleExecError::Cancelled));
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
    let result = tokio::time::timeout(Duration::from_secs(16), engine.search(&id, "x", 1, token))
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
//
// DeepSeek 复审第 7 项：原测试用 300ms 盲等模拟「任务已开始」，存在轮询竞态导致偶发
// 失败。这里改为：parse 脚本先 `await fetch(/start)`，测试等到 /start 请求到达服务端
// 作为「worker 已开始执行」的确定性信号，再 cancel_scope，消除竞态。

#[tokio::test]
async fn exec_cancelled() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/start"))
        .respond_with(ResponseTemplate::new(200).set_body_string("go"))
        .mount(&server)
        .await;
    let start_url = format!("{}/start", server.uri());

    let engine = test_engine();
    let parse_script = format!(
        "async function parse(chapterUrl) {{ await fetch('{start_url}'); while(true){{}} }}"
    );
    let loaded = engine
        .load_rules(vec![RuleInput::Manifest {
            manifest: make_manifest(
                "取消源",
                "function search(k,p){ return [{title:k,url:'https://example.com/x'}]; }",
            )
            .with_parse(&parse_script),
            origin: RuleOrigin::Builtin,
        }])
        .await;
    assert_eq!(loaded[0].status, RuleStatus::Ready);
    let id = loaded[0].id.clone();

    let token = engine.new_scope_token("play:x");
    let parse_fut = engine.parse(&id, "https://example.com/1", token.clone());
    tokio::pin!(parse_fut);
    // 驱动 parse（让任务真正派发到 worker）并等待 worker 执行到 `fetch /start`——
    // 以 mock 请求到达作为「任务已开始执行」的确定性信号（DeepSeek 复审第 7 项，
    // 替代原先 300ms 盲等/轮询竞态）。若 parse 提前结束说明 worker 未成功执行。
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            tokio::select! {
                r = &mut parse_fut => {
                    panic!("parse 提前结束（worker 未开始执行）: {r:?}");
                }
                _ = tokio::time::sleep(Duration::from_millis(10)) => {}
            }
            let reqs = server.received_requests().await.unwrap();
            if reqs.iter().any(|r| r.url.path() == "/start") {
                break;
            }
        }
    })
    .await
    .expect("parse 应在 10s 内开始执行");

    engine.cancel_scope("play:x");
    let err = tokio::time::timeout(Duration::from_secs(5), parse_fut)
        .await
        .expect("取消应快速返回")
        .unwrap_err();
    assert!(matches!(err, RuleExecError::Cancelled));

    // worker 未被永久占用：轮询 4 次回到原 worker，全部应正常成功
    let all_ok = tokio::time::timeout(Duration::from_secs(8), async {
        for _ in 0..4 {
            let token = CancellationToken::new();
            let items = engine
                .search(&id, "x", 1, token)
                .await
                .expect("worker 应被释放");
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
        .call(
            "function search(k,p){ return std.open('/etc/passwd'); }",
            "search",
            vec![],
        )
        .unwrap_err();
    assert!(
        matches!(err, RuleExecError::ScriptError { .. }),
        "std 不应存在，应抛 ScriptError"
    );
    let err2 = sandbox
        .call(
            "function search(k,p){ return os.system('echo hi'); }",
            "search",
            vec![],
        )
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
        let result = sandbox
            .call(&script, "search", vec![])
            .map_err(|e| format!("{e:?}"));
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
    let items = engine
        .search(&id, "x", 1, token)
        .await
        .expect("fetch 应成功");
    assert_eq!(items[0].title, "hello-world");
    assert_eq!(items[0].url, "https://example.com/200");

    // 审计：UA 必须是 moeplay/<版本号>（由 CARGO_PKG_VERSION 派生）
    let requests = server.received_requests().await.unwrap();
    let ua = requests
        .iter()
        .find(|r| r.url.path() == "/hello")
        .and_then(|r| r.headers.get("user-agent"))
        .and_then(|v| v.to_str().ok());
    assert_eq!(ua, Some(concat!("moeplay/", env!("CARGO_PKG_VERSION"))));
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
    let manifest = make_manifest(
        "自定义源",
        "function search(k,p){ return [{title:k,url:'https://example.com'}]; }",
    );
    std::fs::write(
        &custom_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

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
    let slow_script =
        format!("async function search(k,p) {{ await fetch('{slow_url}'); return []; }}");
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
    let slow_script =
        format!("async function search(k,p) {{ await fetch('{slow_url}'); return []; }}");
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

    // Invalid 规则已注册进内部 map（供列表/置灰/删除），但不参与导出：
    // `all_manifests` 只含 Ready 规则（Kimi K3 复审第 2 项——占位 manifest 不得导出，
    // 否则导出后再导入条数不一致）。
    let manifests = engine.all_manifests();
    assert!(
        !manifests.iter().any(|m| m.name == "坏语法源"),
        "Invalid 规则不应出现在 all_manifests（导出必须排除占位 manifest）"
    );

    // 执行层保持「仅 Ready 可执行」不变量
    let token = CancellationToken::new();
    let err = engine.search(&id, "x", 1, token).await.unwrap_err();
    assert!(matches!(err, RuleExecError::RuleNotFound(_)));

    // 自定义 Invalid 规则仍可按 id 删除（删除链路依赖 map 内注册）
    engine.remove_rule(&id).unwrap();
    let manifests = engine.all_manifests();
    assert!(!manifests.iter().any(|m| m.name == "坏语法源"));
}

// ── 测试：Kimi K3 复审第 1 项——自定义规则 id 稳定性 + 删除链路不「复活」────────

#[tokio::test]
async fn custom_rule_id_stable_across_reloads() {
    // 模拟 rules_import 的落盘约定：自定义规则文件存为 {源文件 stem}.json
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("my_custom_rule.json");
    let manifest = make_manifest(
        "稳定 id 源",
        "function search(k,p){ return [{title:k,url:'https://example.com'}]; }",
    );
    std::fs::write(&file, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();

    // 首次加载（模拟导入后）
    let engine_a = test_engine();
    let first = engine_a
        .load_rules(vec![RuleInput::File {
            path: file.clone(),
            origin: RuleOrigin::Custom,
        }])
        .await;
    assert_eq!(first[0].status, RuleStatus::Ready);
    assert_eq!(first[0].id, "my_custom_rule", "文件规则 id = 文件名 stem");

    // 重启模拟：全新引擎重新加载同一文件 → 同一 id（禁止每次加载重新生成随机 id）
    let engine_b = test_engine();
    let second = engine_b
        .load_rules(vec![RuleInput::File {
            path: file.clone(),
            origin: RuleOrigin::Custom,
        }])
        .await;
    assert_eq!(second[0].id, "my_custom_rule", "重启后 id 必须保持稳定");

    // 就地编辑文件内容（内容 hash 变了）→ id 仍不变：删除链路靠文件名而非内容
    let edited = make_manifest(
        "稳定 id 源",
        "function search(k,p){ return [{title:k,url:'https://example.com/edited'}]; }",
    );
    std::fs::write(&file, serde_json::to_string_pretty(&edited).unwrap()).unwrap();
    let engine_d = test_engine();
    let edited_load = engine_d
        .load_rules(vec![RuleInput::File {
            path: file.clone(),
            origin: RuleOrigin::Custom,
        }])
        .await;
    assert_eq!(edited_load[0].id, "my_custom_rule", "内容编辑不应改变 id");

    // 删除链路：注册表按 id 移除 + 命令层定位文件 {id}.json 删除
    engine_b.remove_rule(&second[0].id).unwrap();
    assert!(
        !engine_b
            .all_manifests()
            .iter()
            .any(|m| m.name == "稳定 id 源"),
        "删除后注册表应移除该规则"
    );
    // 等价于 commands::rules::rules_remove_custom 里 custom_rules_dir().join("{id}.json")
    assert!(file.exists(), "删除前 my_custom_rule.json 应存在");
    std::fs::remove_file(&file).unwrap();

    // 文件已删除 → 重新加载不再出现 Ready 规则（删除链路完整，规则不「复活」）
    let engine_c = test_engine();
    let after_remove = engine_c
        .load_rules(vec![RuleInput::File {
            path: file,
            origin: RuleOrigin::Custom,
        }])
        .await;
    assert_eq!(
        after_remove[0].status,
        RuleStatus::Invalid,
        "文件已删除，规则不应复活"
    );
    assert!(after_remove[0].error.is_some());
}

#[tokio::test]
async fn unparseable_custom_file_uses_file_stem_id() {
    // 无法解析的文件拿不到 manifest → 用文件 stem 作稳定 id，删除链路仍可定位
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("broken_rule.json");
    std::fs::write(&file, "not json {").unwrap();

    let engine = test_engine();
    let loaded = engine
        .load_rules(vec![RuleInput::File {
            path: file,
            origin: RuleOrigin::Custom,
        }])
        .await;
    assert_eq!(loaded[0].status, RuleStatus::Invalid);
    assert_eq!(
        loaded[0].id, "broken_rule",
        "解析失败的文件应以 stem 为稳定 id"
    );
    assert_eq!(
        loaded[0].manifest.name, "broken_rule",
        "占位 manifest 应以 stem 命名"
    );
}

// ── 测试：Kimi K3 复审第 2 项——load_rules 按稳定 id upsert，注册表不累积 ──

#[tokio::test]
async fn load_rules_upsert_does_not_accumulate() {
    let engine = test_engine();
    let dir = tempfile::tempdir().unwrap();
    for i in 0..3 {
        let m = make_manifest(
            &format!("upsert 源{i}"),
            &format!(
                "function search(k,p){{ return [{{title:k,url:'https://example.com/{i}'}}]; }}"
            ),
        );
        let file = dir.path().join(format!("rule_{i}.json"));
        std::fs::write(&file, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    }
    let inputs = file_inputs_from(dir.path(), RuleOrigin::Custom);

    let first = engine.load_rules(inputs.clone()).await;
    assert_eq!(first.len(), 3);
    assert_eq!(engine.all_manifests().len(), 3);

    // 重复加载同一批 → 稳定 id（文件 stem）覆盖旧条目（upsert），all_manifests 不膨胀
    for _ in 0..2 {
        let loaded = engine.load_rules(inputs.clone()).await;
        assert_eq!(loaded.len(), 3);
        assert_eq!(
            engine.all_manifests().len(),
            3,
            "重复加载后 all_manifests 不应膨胀"
        );
    }

    // 幂等：每次加载 id 仍是文件 stem（无随机 id 累积）
    let loaded = engine.load_rules(inputs.clone()).await;
    let mut ids: Vec<&str> = loaded.iter().map(|r| r.id.as_str()).collect();
    ids.sort_unstable();
    assert_eq!(ids, vec!["rule_0", "rule_1", "rule_2"]);
}

#[tokio::test]
async fn manifest_input_reload_upserts_same_id() {
    let engine = test_engine();
    let manifest = make_manifest("upsert 源", "function search(k,p){ return []; }");

    let first = engine
        .load_rules(vec![RuleInput::Manifest {
            manifest: manifest.clone(),
            origin: RuleOrigin::Builtin,
        }])
        .await;
    let second = engine
        .load_rules(vec![RuleInput::Manifest {
            manifest: manifest.clone(),
            origin: RuleOrigin::Builtin,
        }])
        .await;

    assert_eq!(
        first[0].id, second[0].id,
        "同一 manifest 重复加载应得同一 id"
    );
    assert_eq!(
        engine.all_manifests().len(),
        1,
        "注册表应按稳定 id 覆盖，不累积"
    );
}

// ── 测试：Kimi K3 复审第 2 项——导出排除 Invalid 占位 manifest，往返条数一致 ──

#[tokio::test]
async fn export_excludes_invalid_placeholder_manifests() {
    let engine = test_engine();
    let dir = tempfile::tempdir().unwrap();

    // 一条合法自定义规则（可导出）
    let good_path = dir.path().join("good_rule.json");
    let good = make_manifest(
        "好规则",
        "function search(k,p){ return [{title:k,url:'https://example.com'}]; }",
    );
    std::fs::write(&good_path, serde_json::to_string_pretty(&good).unwrap()).unwrap();
    let good_loaded = engine
        .load_rules(vec![RuleInput::File {
            path: good_path,
            origin: RuleOrigin::Custom,
        }])
        .await;
    assert_eq!(good_loaded[0].status, RuleStatus::Ready);

    // 一条解析失败文件（→ Invalid 占位 manifest：空 baseUrl/search 等，不可导出）
    let bad_path = dir.path().join("broken_rule.json");
    std::fs::write(&bad_path, "not json {").unwrap();
    let bad_loaded = engine
        .load_rules(vec![RuleInput::File {
            path: bad_path,
            origin: RuleOrigin::Custom,
        }])
        .await;
    assert_eq!(bad_loaded[0].status, RuleStatus::Invalid);

    // all_manifests 只含 Ready 规则（占位 manifest 不得导出）
    let manifests = engine.all_manifests();
    assert_eq!(
        manifests.len(),
        1,
        "导出清单应排除 Invalid 占位 manifest，实际含: {:?}",
        manifests.iter().map(|m| m.name.clone()).collect::<Vec<_>>()
    );
    assert_eq!(manifests[0].name, "好规则");

    // 往返：导出 JSON → 重新加载 → Ready 条数一致（spec §6.1 测试 13）
    let exported = serde_json::to_string_pretty(&manifests).unwrap();
    let parsed: Vec<RuleManifest> = serde_json::from_str(&exported).unwrap();
    let reloaded = engine
        .load_rules(
            parsed
                .into_iter()
                .map(|manifest| RuleInput::Manifest {
                    manifest,
                    origin: RuleOrigin::Custom,
                })
                .collect(),
        )
        .await;
    let ready = reloaded
        .iter()
        .filter(|r| r.status == RuleStatus::Ready)
        .count();
    assert_eq!(
        ready,
        manifests.len(),
        "导出后再导入 Ready 条数应一致（占位条目不得破坏往返计数）"
    );
}

// ── 测试：Kimi K3 复审第 3 项——per-call token 不取消同规则其他并发调用 ──

#[tokio::test]
async fn per_call_token_does_not_cancel_siblings() {
    let engine = test_engine();

    // 两个独立 per-call token：互不取消，cancel_scope 也够不到（未注册进 scope 表）
    let t1 = engine.per_call_token();
    let t2 = engine.per_call_token();
    assert!(!t1.is_cancelled(), "per-call token 不应被其他调用取消");
    assert!(!t2.is_cancelled(), "per-call token 不应被其他调用取消");

    // 同一规则上的并发 search（模拟翻页 page=1 / page=2）：两个 token 独立，均可完成
    let loaded = engine
        .load_rules(vec![RuleInput::Manifest {
            manifest: make_manifest(
                "翻页源",
                "function search(k,p){ return [{title:k,url:'https://example.com/'+p}]; }",
            ),
            origin: RuleOrigin::Builtin,
        }])
        .await;
    assert_eq!(loaded[0].status, RuleStatus::Ready);
    let id = loaded[0].id.clone();

    let (r1, r2) = tokio::join!(
        engine.search(&id, "x", 1, t1.clone()),
        engine.search(&id, "x", 2, t2.clone()),
    );
    let ok1 = r1.expect("page=1 search 应成功（不被并发 page=2 取消）");
    let ok2 = r2.expect("page=2 search 应成功（不被并发 page=1 取消）");
    assert_eq!(ok1[0].url, "https://example.com/1");
    assert_eq!(ok2[0].url, "https://example.com/2");

    // per-call token 未注册进 scope 表：cancel_scope 同名 scope 不影响它们
    engine.cancel_scope("search:翻页源");
    assert!(
        !t1.is_cancelled(),
        "cancel_scope 不应取消未注册的 per-call token"
    );
    assert!(!t2.is_cancelled());
}

// ── 测试：Kimi K3 复审项——硬中断迟到安装不污染下一任务 ────────────────────
//
// 复现竞态：取消任务 A 时 `hard_interrupt_js` 用 spawn_blocking 分离安装恒 true 处理器；
// 若该安装延迟到 worker 已为下一任务 `rearm_interrupt` 之后才执行，会把 always-true 中断
// 装到下一任务上，令其被无条件中断。本测试注入 600ms 安装延迟（`with_hard_interrupt_delay`），
// 让迟到的硬中断落在任务 B 执行期间安装，断言任务 B 不受影响（仍在运行、可被正常取消），
// 任务 A 正常返回 Cancelled。

#[tokio::test]
async fn hard_interrupt_late_install_does_not_poison_next_task() {
    let engine = Arc::new(test_engine().with_hard_interrupt_delay(Duration::from_millis(600)));

    // 阻塞规则：while(true) 死循环，用于被取消（任务 A）与检测污染（任务 B）
    let block = engine
        .load_rules(vec![RuleInput::Manifest {
            manifest: make_manifest("阻塞源", "function search(k,p){ while(true){} }"),
            origin: RuleOrigin::Builtin,
        }])
        .await;
    assert_eq!(block[0].status, RuleStatus::Ready);
    let block_id = block[0].id.clone();

    // 快速规则：推进 round-robin，使任务 B 回落同一槽位（0）
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

    // 任务 A → 槽位 0（round-robin 计数器 0）
    let token_a = engine.new_scope_token("race:a");
    let handle_a = {
        let engine = Arc::clone(&engine);
        let id = block_id.clone();
        tokio::spawn(async move { engine.search(&id, "x", 1, token_a.clone()).await })
    };
    tokio::time::sleep(Duration::from_millis(100)).await; // 等任务 A 开始执行
    engine.cancel_scope("race:a"); // 触发硬中断（600ms 后安装，注入延迟）

    // 推进 round-robin：3 个快速任务占槽位 1/2/3，使下一次派发回落槽位 0
    for _ in 0..3 {
        let token = CancellationToken::new();
        engine
            .search(&fast_id, "x", 1, token)
            .await
            .expect("快速任务应成功");
    }

    // 任务 B → 槽位 0：worker 完成任务 A 并 rearm 后开始执行
    let token_b = engine.new_scope_token("race:b");
    let mut handle_b = {
        let engine = Arc::clone(&engine);
        let id = block_id.clone();
        tokio::spawn(async move { engine.search(&id, "x", 1, token_b.clone()).await })
    };
    tokio::time::sleep(Duration::from_millis(100)).await; // 等任务 B 开始执行

    // 越过硬中断安装延迟：迟到的硬中断现在落在任务 B 执行期间安装
    tokio::time::sleep(Duration::from_millis(700)).await;

    // 任务 B 不应被污染：短超时探测其是否仍在运行（未被异常中断而提前 resolve）
    let still_running = tokio::time::timeout(Duration::from_millis(80), async {
        loop {
            tokio::select! {
                r = &mut handle_b => return r,
                _ = tokio::time::sleep(Duration::from_millis(5)) => {}
            }
        }
    })
    .await;
    if let Ok(res) = still_running {
        panic!("任务 B 被迟到的硬中断污染: {res:?}");
    }

    // 任务 A 应正常返回 Cancelled
    let err_a = tokio::time::timeout(Duration::from_secs(5), handle_a)
        .await
        .expect("任务 A 取消应在超时内返回")
        .expect("任务 A 不应 panic")
        .unwrap_err();
    assert!(matches!(err_a, RuleExecError::Cancelled));

    // 任务 B 取消后应正常返回 Cancelled（证明未被污染、仍受取消控制）
    engine.cancel_scope("race:b");
    let err_b = tokio::time::timeout(Duration::from_secs(5), handle_b)
        .await
        .expect("任务 B 取消应在超时内返回")
        .expect("任务 B 不应 panic")
        .unwrap_err();
    assert!(matches!(err_b, RuleExecError::Cancelled));
}

// ── 测试：Kimi K3 复审第 4 项——rules_import 同名 stem 禁止静默覆盖 ─────────

#[tokio::test]
async fn import_same_stem_appends_suffix_not_overwrite() {
    let engine = test_engine();
    let root = tempfile::tempdir().unwrap();
    let src_dir = root.path().join("src");
    let dst_dir = root.path().join("custom_rules");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();

    // 两份内容不同、文件名 stem 相同的规则文件（分处不同目录，模拟两次独立导入）
    let src1 = src_dir.join("dup.json");
    let src2 = root.path().join("src2").join("dup.json");
    std::fs::create_dir_all(src2.parent().unwrap()).unwrap();
    let m1 = make_manifest(
        "同名源A",
        "function search(k,p){ return [{title:k,url:'https://example.com/a'}]; }",
    );
    let m2 = make_manifest(
        "同名源B",
        "function search(k,p){ return [{title:k,url:'https://example.com/b'}]; }",
    );
    std::fs::write(&src1, serde_json::to_string_pretty(&m1).unwrap()).unwrap();
    std::fs::write(&src2, serde_json::to_string_pretty(&m2).unwrap()).unwrap();

    // 第一次导入：无冲突 → 落盘 dup.json，id = "dup"
    let first = import_rule_to_dir(&engine, &src1, &dst_dir).await.unwrap();
    assert_eq!(first.id, "dup");
    assert!(dst_dir.join("dup.json").exists());

    // 第二次导入同 stem：目标已存在且 id 已注册 → 自动追加 -2，绝不覆盖
    let second = import_rule_to_dir(&engine, &src2, &dst_dir).await.unwrap();
    assert_eq!(second.id, "dup-2");
    assert!(
        dst_dir.join("dup.json").exists(),
        "首次导入的 dup.json 不得被静默覆盖"
    );
    assert!(dst_dir.join("dup-2.json").exists());
    assert!(engine.is_registered("dup") && engine.is_registered("dup-2"));

    // 原文件内容保持不变（未被第二次导入的内容静默替换）
    let on_disk: RuleManifest =
        serde_json::from_slice(&std::fs::read(dst_dir.join("dup.json")).unwrap()).unwrap();
    assert_eq!(on_disk.name, "同名源A");

    // 第三次同 stem → -3
    let src3 = root.path().join("src3").join("dup.json");
    std::fs::create_dir_all(src3.parent().unwrap()).unwrap();
    std::fs::write(&src3, serde_json::to_string_pretty(&m1).unwrap()).unwrap();
    let third = import_rule_to_dir(&engine, &src3, &dst_dir).await.unwrap();
    assert_eq!(third.id, "dup-3");
    assert!(dst_dir.join("dup-3.json").exists());
}

#[tokio::test]
async fn import_unique_id_considers_registry_and_disk() {
    let engine = test_engine();
    let dir = tempfile::tempdir().unwrap();

    // 空引擎 + 空目录：直接用 stem
    assert_eq!(unique_custom_rule_id(&engine, "rule", dir.path()), "rule");

    // 磁盘已有 rule.json（即使引擎未注册）→ 也须避开，禁止静默覆盖文件
    std::fs::write(dir.path().join("rule.json"), "{}").unwrap();
    assert!(!engine.is_registered("rule"));
    assert_eq!(unique_custom_rule_id(&engine, "rule", dir.path()), "rule-2");

    // 注册表已有 rule-2 → 继续追加 -3（注册表冲突同样触发后缀）
    engine.register_loaded(LoadedRule {
        id: "rule-2".into(),
        manifest: make_manifest("已注册源", "function search(k,p){ return []; }"),
        origin: RuleOrigin::Custom,
        status: RuleStatus::Ready,
        error: None,
    });
    assert_eq!(unique_custom_rule_id(&engine, "rule", dir.path()), "rule-3");
}

// ── 辅助扩展 ────────────────────────────────────────────────────────────

/// 扫描目录内全部规则文件为 File 输入（等价于 commands::rules::discover_rule_inputs）。
fn file_inputs_from(dir: &std::path::Path, origin: RuleOrigin) -> Vec<RuleInput> {
    std::fs::read_dir(dir)
        .expect("目录应可读")
        .flatten()
        .map(|e| e.path())
        .filter(|p| RuleFileFormat::from_path(p).is_some())
        .map(|p| RuleInput::File { path: p, origin })
        .collect()
}

impl RuleManifest {
    fn with_parse(mut self, parse: &str) -> Self {
        self.parse = parse.to_string();
        self
    }
}
