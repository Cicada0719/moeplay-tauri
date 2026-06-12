// 萌游 MoeGame · 结构化日志与崩溃捕获
//
// 使用 `tracing` 替代 `log` + `eprintln!`，提供：
//   - 文件滚动日志（保留最近 7 天）
//   - 控制台输出（开发调试）
//   - panic hook 崩溃落盘
//   - 诊断导出时可收集

use std::fs;
use std::path::PathBuf;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// 获取日志目录
pub fn log_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("moeplay")
        .join("logs")
}

/// 初始化 tracing：文件滚动 + 控制台 + panic hook。
pub fn init() {
    let dir = log_dir();
    fs::create_dir_all(&dir).ok();

    // 文件滚动：每天一个文件，保留 7 天
    let file_appender = tracing_appender::rolling::daily(&dir, "moegame.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // 过滤器：默认 info，可设 RUST_LOG=debug
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = fmt::layer().with_target(true).with_thread_ids(true).json();

    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer.with_writer(non_blocking))
        .with(
            fmt::layer()
                .with_target(false)
                .with_thread_names(true)
                .pretty()
                .with_writer(std::io::stderr),
        );

    tracing::subscriber::set_global_default(subscriber)
        .expect("tracing subscriber should be set only once");

    // 注意：_guard 需要泄漏（全局存活），否则 non_blocking writer 会被 drop
    std::mem::forget(_guard);

    // Panic hook：崩溃时记录完整信息和回溯
    std::panic::set_hook(Box::new(|info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown".to_string());
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown panic".to_string());
        let backtrace = std::backtrace::Backtrace::force_capture();

        tracing::error!(
            location = %location,
            payload = %payload,
            backtrace = %backtrace,
            "PANIC"
        );

        // 也写到 stderr 确保可见
        eprintln!("!!! PANIC at {location}: {payload}");
        eprintln!("{backtrace}");
    }));

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "MoeGame tracing initialized"
    );
}

/// 收集最近日志用于诊断导出。
pub fn collect_recent_logs(lines: usize) -> Vec<String> {
    let dir = log_dir();
    let mut entries: Vec<_> = fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "log")
                .unwrap_or(false)
        })
        .collect();
    entries.sort_by_key(|e| e.path());
    entries.reverse(); // 最新在前

    let mut result = Vec::new();
    for entry in entries.iter().take(3) {
        // 只读最近3个文件
        if let Ok(content) = fs::read_to_string(entry.path()) {
            let file_lines: Vec<&str> = content.lines().collect();
            let start = if file_lines.len() > lines {
                file_lines.len() - lines
            } else {
                0
            };
            for line in &file_lines[start..] {
                result.push(line.to_string());
            }
        }
    }
    result
}
