// 萌游 MoeGame · 进程监控（M2 游玩计时）
//
// 跟踪已启动的游戏子进程，检测进程退出后自动结束游玩会话。
// 设计：stored Child handle → tokio spawn_blocking wait → 进程退出时自动 end_session。
// 通过事件 `play-session-ended` 通知前端。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::Emitter;
use tauri::Manager;

/// 正在运行的游戏追踪记录
struct RunningEntry {
    session_id: String,
    game_id: String,
    started_at: std::time::Instant,
}

/// 进程监视器。被 Tauri `.manage()` 注入。
pub struct ProcessMonitor {
    running: Mutex<HashMap<u32, RunningEntry>>,
}

impl ProcessMonitor {
    pub fn new() -> Self {
        Self {
            running: Mutex::new(HashMap::new()),
        }
    }

    /// 注册一个正在运行的游戏进程。
    pub fn register(&self, child_id: u32, session_id: &str, game_id: &str) {
        let mut running = self.running.lock().unwrap();
        running.insert(
            child_id,
            RunningEntry {
                session_id: session_id.to_string(),
                game_id: game_id.to_string(),
                started_at: std::time::Instant::now(),
            },
        );
        tracing::info!(child_id, session_id, game_id, "Registered running game");
    }

    /// 进程退出回调：自动结束会话并推送事件。
    pub fn on_exit(&self, child_id: u32, app_handle: &tauri::AppHandle) {
        let entry = {
            let mut running = self.running.lock().unwrap();
            running.remove(&child_id)
        };
        if let Some(entry) = entry {
            let elapsed = entry.started_at.elapsed().as_secs();
            tracing::info!(
                child_id,
                session_id = %entry.session_id,
                game_id = %entry.game_id,
                elapsed_secs = elapsed,
                "Game process exited"
            );

            // 持久化
            let db = app_handle.state::<crate::db::Database>();
            let _ = db.end_play_session(&entry.game_id, &entry.session_id, elapsed);

            // 通知前端
            let exited = ExitedGame {
                session_id: entry.session_id,
                game_id: entry.game_id,
                duration_seconds: elapsed,
            };
            let _ = app_handle.emit("play-session-ended", &exited);
        }
    }

    /// 获取当前运行中的游戏数量。
    pub fn running_count(&self) -> usize {
        self.running.lock().unwrap().len()
    }

    /// 获取正在运行的游戏列表。
    pub fn running_games(&self) -> Vec<RunningGameInfo> {
        let running = self.running.lock().unwrap();
        running
            .iter()
            .map(|(pid, entry)| RunningGameInfo {
                session_id: entry.session_id.clone(),
                game_id: entry.game_id.clone(),
                elapsed_seconds: entry.started_at.elapsed().as_secs(),
                pid: *pid,
            })
            .collect()
    }

    /// 手动标记某个 session 已结束。
    pub fn unregister_by_session(&self, session_id: &str) {
        let mut running = self.running.lock().unwrap();
        running.retain(|_, e| e.session_id != session_id);
    }
}

impl Default for ProcessMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// 游戏退出事件数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExitedGame {
    pub session_id: String,
    pub game_id: String,
    pub duration_seconds: u64,
}

/// 正在运行的游戏信息（供前端展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningGameInfo {
    pub session_id: String,
    pub game_id: String,
    pub elapsed_seconds: u64,
    pub pid: u32,
}

/// 后台等待子进程退出，退出时调用 monitor.on_exit()。
///
/// `launch_game` 是同步 Tauri 命令，Windows 上执行它的线程不保证位于 Tokio
/// runtime 内。这里不能直接调用 `tokio::task::spawn_blocking`，否则会在游戏已经
/// 启动后触发 "there is no reactor running" panic，并跨 Tauri 命令边界终止整个应用。
pub fn spawn_exit_watcher(
    mut child: std::process::Child,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let child_id = child.id();
    spawn_wait_thread(format!("moeplay-game-wait-{child_id}"), move || {
        if let Err(error) = child.wait() {
            tracing::warn!(child_id, %error, "Failed to wait for game process");
        }
        let monitor = app_handle.state::<ProcessMonitor>();
        monitor.on_exit(child_id, &app_handle);
    })
    .map_err(|error| format!("无法启动游戏进程监控线程: {error}"))
}

fn spawn_wait_thread(name: String, waiter: impl FnOnce() + Send + 'static) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name(name)
        .spawn(waiter)
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::spawn_wait_thread;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn process_wait_thread_does_not_require_a_tokio_runtime() {
        let (sender, receiver) = mpsc::channel();

        // This test intentionally runs as a plain Rust test without creating a Tokio runtime.
        // The 0.19.2 implementation panicked here when it used tokio::task::spawn_blocking.
        spawn_wait_thread(
            "moeplay-process-monitor-regression".to_string(),
            move || sender.send(()).unwrap(),
        )
        .expect("process monitor thread should start without a Tokio runtime");

        receiver
            .recv_timeout(Duration::from_secs(2))
            .expect("process monitor thread should run");
    }
}
