//! 源健康检查 CLI（spec task-02 Step 5，供 CI 每日探测）。
//!
//! 无 GUI 依赖：直接构造 headless `RuleEngine`，加载 `resources/rules/`，
//! 对每条规则执行一次真实搜索探测（`probeKeyword`），输出 JSON 报告：
//! `{ "status": "probe-complete", "total": N, "passed": M, "passRate": 0.xx,
//! "failures": [{ "id", "stage", "errorKind", "httpStatus", "error" }] }`。
//!
//! 退出码：`passRate < 0.8` → exit 1（CI job 失败 → 触发告警 issue）；否则 exit 0。
//!
//! 用法：
//! ```text
//! cargo run --release --bin rules-health -- --rules-dir ../resources/rules --out ../health-report.json
//! ```

use std::path::PathBuf;

use moeplay_lib::rules::{self, health, schema::RuleStatus};

fn parse_args() -> (PathBuf, PathBuf) {
    let mut rules_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("rules");
    let mut out = PathBuf::from("health-report.json");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--rules-dir" => {
                rules_dir = PathBuf::from(args.next().expect("--rules-dir 需要目录参数"))
            }
            "--out" => out = PathBuf::from(args.next().expect("--out 需要路径参数")),
            "--help" | "-h" => {
                println!("用法: rules-health [--rules-dir <目录>] [--out <路径>]");
                std::process::exit(0);
            }
            other => {
                eprintln!("未知参数: {other}");
                std::process::exit(2);
            }
        }
    }
    (rules_dir, out)
}

#[tokio::main]
async fn main() {
    let (rules_dir, out) = parse_args();
    if !rules_dir.is_dir() {
        eprintln!("[rules-health] 规则目录不存在: {}", rules_dir.display());
        std::process::exit(2);
    }

    let engine = rules::new_headless();
    let loaded = rules::load_rules_from_dir(&engine, &rules_dir).await;
    let ready = loaded
        .iter()
        .filter(|r| r.status == RuleStatus::Ready)
        .count();
    eprintln!(
        "[rules-health] 规则加载: Ready {ready}/{}（目录 {}）",
        loaded.len(),
        rules_dir.display()
    );

    let targets = health::discover_rules(&rules_dir);
    if targets.is_empty() {
        eprintln!("[rules-health] 未发现任何待探测源");
        write_report(&out, 0, 0, &[]);
        std::process::exit(1);
    }

    let mut failures: Vec<serde_json::Value> = Vec::new();
    let mut passed = 0usize;
    let mut total = 0usize;
    for t in targets {
        total += 1;
        let res = health::probe_one(&engine, &t.id, &t.keyword).await;
        if res.ok {
            passed += 1;
            eprintln!("[rules-health] [ok]   {} ({:?})", t.id, res.latency_ms);
        } else {
            let err = res.error.clone().unwrap_or_else(|| "未知错误".to_string());
            eprintln!("[rules-health] [fail] {}: {}", t.id, err);
            failures.push(serde_json::json!({
                "id": t.id,
                "stage": res.stage,
                "errorKind": res.error_kind,
                "httpStatus": res.http_status,
                "checkedAt": res.checked_at,
                "error": err,
            }));
        }
    }

    let pass_rate = passed as f64 / total as f64;
    write_report(&out, total, passed, &failures);
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "probe-complete",
            "total": total,
            "passed": passed,
            "passRate": pass_rate,
            "failures": failures,
        }))
        .unwrap_or_default()
    );

    if pass_rate < 0.8 {
        eprintln!("[rules-health] 通过率 {pass_rate:.2} < 0.8，退出码 1（触发 CI 告警）");
        std::process::exit(1);
    }
}

fn write_report(out: &PathBuf, total: usize, passed: usize, failures: &[serde_json::Value]) {
    let pass_rate = if total == 0 {
        0.0
    } else {
        passed as f64 / total as f64
    };
    let report = serde_json::json!({
        "status": "probe-complete",
        "failureKind": if failures.is_empty() { serde_json::Value::Null } else { serde_json::json!("probe") },
        "total": total,
        "passed": passed,
        "passRate": pass_rate,
        "failures": failures,
    });
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    match serde_json::to_string_pretty(&report)
        .map_err(|e| e.to_string())
        .and_then(|json| std::fs::write(out, json).map_err(|e| e.to_string()))
    {
        Ok(()) => eprintln!("[rules-health] 报告已写入: {}", out.display()),
        Err(e) => eprintln!("[rules-health] 写入报告失败: {e}"),
    }
}
