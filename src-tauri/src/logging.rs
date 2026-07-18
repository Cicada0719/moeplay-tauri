// 萌游 MoeGame · 结构化日志与崩溃捕获
//
// 使用 `tracing` 替代 `log` + `eprintln!`，提供：
//   - 文件滚动日志（保留最近 7 天）
//   - 控制台输出（开发调试）
//   - panic hook 崩溃落盘
//   - 诊断导出时可收集

use std::fs;
use std::path::{Path, PathBuf};
#[cfg(not(mobile))]
use std::time::{Duration, SystemTime};
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
    #[cfg(not(mobile))]
    let dir = log_dir();
    #[cfg(not(mobile))]
    fs::create_dir_all(&dir).ok();
    #[cfg(not(mobile))]
    prune_log_files(&dir);

    // 文件滚动：每天一个文件，保留 7 天
    #[cfg(not(mobile))]
    let file_appender = tracing_appender::rolling::daily(&dir, "moegame.log");
    #[cfg(not(mobile))]
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // 过滤器：默认 info，可设 RUST_LOG=debug
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    #[cfg(not(mobile))]
    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .json()
                .with_writer(non_blocking),
        )
        .with(
            fmt::layer()
                .with_target(false)
                .with_thread_names(true)
                .pretty()
                .with_writer(std::io::stderr),
        );

    // Android starts before a Tauri AppHandle is available. At that point
    // `dirs::data_dir()` can resolve to the read-only filesystem root, and
    // tracing-appender panics while creating its rolling log directory. Logcat
    // already captures stderr, so mobile startup deliberately uses only the
    // console layer.
    #[cfg(mobile)]
    let subscriber = tracing_subscriber::registry().with(env_filter).with(
        fmt::layer()
            .with_target(false)
            .with_thread_names(true)
            .pretty()
            .with_writer(std::io::stderr),
    );

    tracing::subscriber::set_global_default(subscriber)
        .expect("tracing subscriber should be set only once");

    // 注意：_guard 需要泄漏（全局存活），否则 non_blocking writer 会被 drop
    #[cfg(not(mobile))]
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
        .filter(|entry| is_log_file(&entry.path()))
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

#[cfg(not(mobile))]
const LOG_RETENTION: Duration = Duration::from_secs(7 * 24 * 60 * 60);
#[cfg(not(mobile))]
const MAX_LOG_BYTES: u64 = 100 * 1024 * 1024;

fn is_log_file(path: &Path) -> bool {
    path.is_file()
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("moegame.log"))
}

#[cfg(not(mobile))]
fn prune_log_files(dir: &Path) {
    prune_log_files_with_policy(dir, SystemTime::now(), LOG_RETENTION, MAX_LOG_BYTES);
}

#[cfg(not(mobile))]
fn prune_log_files_with_policy(dir: &Path, now: SystemTime, retention: Duration, max_bytes: u64) {
    let mut files: Vec<_> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| is_log_file(path))
        .filter_map(|path| fs::metadata(&path).ok().map(|metadata| (path, metadata)))
        .collect();

    for (path, metadata) in &files {
        let expired = metadata
            .modified()
            .ok()
            .and_then(|modified| now.duration_since(modified).ok())
            .is_some_and(|age| age > retention);
        if expired {
            let _ = fs::remove_file(path);
        }
    }

    files.retain(|(path, _)| path.exists());
    files.sort_by_key(|(_, metadata)| metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH));
    files.reverse();
    let mut retained = 0_u64;
    for (path, metadata) in files {
        retained = retained.saturating_add(metadata.len());
        if retained > max_bytes {
            let _ = fs::remove_file(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("moeplay-log-policy-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn log_policy_uses_daily_names_and_enforces_quota() {
        let dir = temp_dir();
        fs::write(dir.join("moegame.log.2026-07-09"), b"12345678").unwrap();
        fs::write(dir.join("moegame.log.2026-07-10"), b"abcdefgh").unwrap();
        fs::write(dir.join("unrelated.log"), b"keep").unwrap();
        prune_log_files_with_policy(&dir, SystemTime::now(), Duration::from_secs(3600), 8);
        let app_logs = fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| is_log_file(&entry.path()))
            .count();
        assert_eq!(app_logs, 1);
        assert!(dir.join("unrelated.log").exists());
        let _ = fs::remove_dir_all(dir);
    }
}
