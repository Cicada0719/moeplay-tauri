// 萌游 MoeGame - 目录监听自动导入模块
//
// 使用 notify crate 监控配置的目录变化，
// 自动发现新游戏文件夹，调用刮削和入库管线。

use crate::db::Database;
use crate::models::Game;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    mpsc::{self, Sender},
    Arc, Mutex,
};
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};

/// 导入事件 —— 通过 Tauri 事件发送给前端
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImportEvent {
    ScanStarted {
        dir: String,
    },
    GameFound {
        name: String,
        path: String,
    },
    GameImported {
        game_id: String,
        name: String,
    },
    ArchiveFound {
        name: String,
        path: String,
    },
    ArchiveExtracted {
        name: String,
        output: String,
        game_root: Option<String>,
    },
    ScanFinished {
        dir: String,
        imported: usize,
        skipped: usize,
    },
    FileChanged {
        path: String,
        kind: String,
    },
    ScrapeQueued {
        game_id: String,
        name: String,
    },
    Error {
        message: String,
    },
}

/// 目录监听的托管状态
pub struct ImportWatcher {
    /// 关闭信号发送端
    shutdown_tx: Mutex<Option<Sender<()>>>,
    /// 是否正在运行
    is_running: Arc<Mutex<bool>>,
    /// 上次事件处理时间（用于防抖）
    last_event: Arc<Mutex<Option<Instant>>>,
    /// 延迟处理间隔（毫秒）
    debounce_ms: u64,
}

impl ImportWatcher {
    /// 创建新的导入监听器
    pub fn new() -> Self {
        Self {
            shutdown_tx: Mutex::new(None),
            is_running: Arc::new(Mutex::new(false)),
            last_event: Arc::new(Mutex::new(None)),
            debounce_ms: 500,
        }
    }

    /// 检查监听器是否正在运行
    pub fn is_running(&self) -> bool {
        *self.is_running.lock().unwrap()
    }

    /// 启动目录监听
    ///
    /// 为每个监控目录设置 notify watcher，在独立线程中处理文件变化事件。
    pub fn start(
        &self,
        app_handle: tauri::AppHandle,
        watch_dirs: Vec<String>,
    ) -> Result<(), String> {
        if watch_dirs.is_empty() {
            return Err("没有配置监控目录".to_string());
        }

        // 如果已经在运行，先停止
        if self.is_running() {
            self.stop();
        }

        let is_running = self.is_running.clone();
        let last_event = self.last_event.clone();
        let debounce_ms = self.debounce_ms;

        // 创建关闭通道
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();

        // 保存关闭发送端
        {
            let mut tx = self.shutdown_tx.lock().unwrap();
            *tx = Some(shutdown_tx);
        }

        // 标记为运行中
        *is_running.lock().unwrap() = true;

        // 在独立线程中运行 watcher
        std::thread::spawn(move || {
            // 先对每个监控目录执行一次完整扫描
            for dir in &watch_dirs {
                let dir_path = PathBuf::from(dir);
                if dir_path.is_dir() {
                    let _ = app_handle.emit(
                        "import-event",
                        ImportEvent::ScanStarted { dir: dir.clone() },
                    );

                    let (imported, skipped) = scan_directory(&app_handle, &dir_path);
                    let _ = app_handle.emit(
                        "import-event",
                        ImportEvent::ScanFinished {
                            dir: dir.clone(),
                            imported,
                            skipped,
                        },
                    );
                }
            }

            // 创建 notify watcher
            let (event_tx, event_rx) = mpsc::channel::<Result<Event, notify::Error>>();

            let mut watcher = match RecommendedWatcher::new(
                move |res| {
                    let _ = event_tx.send(res);
                },
                Config::default(),
            ) {
                Ok(w) => w,
                Err(e) => {
                    let _ = app_handle.emit(
                        "import-event",
                        ImportEvent::Error {
                            message: format!("无法创建目录监听器: {}", e),
                        },
                    );
                    *is_running.lock().unwrap() = false;
                    return;
                }
            };

            // 监听所有配置目录（非递归：只监听顶层，子目录变化通过手动扫描处理）
            for dir in &watch_dirs {
                let dir_path = PathBuf::from(dir);
                if dir_path.is_dir() {
                    if let Err(e) = watcher.watch(&dir_path, RecursiveMode::NonRecursive) {
                        let _ = app_handle.emit(
                            "import-event",
                            ImportEvent::Error {
                                message: format!("监听目录失败 {}: {}", dir, e),
                            },
                        );
                    }
                    // 同时递归监听子目录（已在 scan_directory 中处理过的）
                    if let Err(e) = watcher.watch(&dir_path, RecursiveMode::Recursive) {
                        let _ = app_handle.emit(
                            "import-event",
                            ImportEvent::Error {
                                message: format!("递归监听目录失败 {}: {}", dir, e),
                            },
                        );
                    }
                }
            }

            // 事件处理循环
            let mut pending_events: Vec<PathBuf> = Vec::new();
            let debounce_duration = Duration::from_millis(debounce_ms);

            loop {
                // 检查关闭信号
                if shutdown_rx.try_recv().is_ok() {
                    break;
                }

                // 接收事件（带超时以便定期检查关闭信号）
                match event_rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(Ok(event)) => {
                        let now = Instant::now();
                        let should_process = {
                            let last = last_event.lock().unwrap();
                            if let Some(t) = *last {
                                now.duration_since(t) > debounce_duration
                                    && !pending_events.is_empty()
                            } else {
                                false
                            }
                        };

                        // 收集事件中的路径
                        for path in &event.paths {
                            if !pending_events.contains(path) {
                                pending_events.push(path.clone());
                            }
                        }

                        // 更新最后事件时间
                        *last_event.lock().unwrap() = Some(now);

                        // 通知前端文件变化
                        let _ = app_handle.emit(
                            "import-event",
                            ImportEvent::FileChanged {
                                path: event
                                    .paths
                                    .first()
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or_default(),
                                kind: format!("{:?}", event.kind),
                            },
                        );

                        // 如果是 create 事件且未在处理中，延迟处理
                        if should_process || matches!(event.kind, EventKind::Create(_)) {
                            // 标记为已处理
                            if pending_events.is_empty() {
                                // 启动延迟处理
                                let paths = std::mem::take(&mut pending_events);
                                let handle = app_handle.clone();
                                let last = last_event.clone();
                                let dur = debounce_duration;
                                std::thread::spawn(move || {
                                    std::thread::sleep(dur);
                                    let _ = handle_deferred_events(&handle, &paths);
                                    *last.lock().unwrap() = None;
                                });
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        let _ = app_handle.emit(
                            "import-event",
                            ImportEvent::Error {
                                message: format!("监听错误: {}", e),
                            },
                        );
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        // 超时，继续循环检查关闭信号
                        // 如果有积压事件且超过防抖时间，处理它们
                        if !pending_events.is_empty() {
                            let should_flush = last_event
                                .lock()
                                .unwrap()
                                .map(|t| t.elapsed() > debounce_duration)
                                .unwrap_or(true);
                            if should_flush {
                                let paths = std::mem::take(&mut pending_events);
                                let handle = app_handle.clone();
                                let _ = handle_deferred_events(&handle, &paths);
                                *last_event.lock().unwrap() = None;
                            }
                        }
                        continue;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        break;
                    }
                }
            }

            // 清理
            *is_running.lock().unwrap() = false;
            log::info!("Import watcher stopped");
        });

        Ok(())
    }

    /// 停止目录监听
    pub fn stop(&self) {
        let mut tx = self.shutdown_tx.lock().unwrap();
        if let Some(sender) = tx.take() {
            let _ = sender.send(());
        }
        *self.is_running.lock().unwrap() = false;
    }
}

impl Default for ImportWatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// 处理延迟的事件（防抖后统一处理）
fn handle_deferred_events(app_handle: &tauri::AppHandle, paths: &[PathBuf]) -> Result<(), String> {
    for path in paths {
        if !path.exists() {
            continue;
        }
        if path.is_file() {
            if is_executable(path) && !is_skip_exe(path) {
                import_game_from_path(app_handle, path)?;
            } else if is_archive(path) {
                // M4: 自动解压→入库管线
                process_archive(app_handle, path);
            }
        } else if path.is_dir() {
            scan_directory(app_handle, path);
        }
    }
    Ok(())
}

/// M4 压缩包处理管线：解压 → 定位游戏根 → 智能选择 exe → 入库 → 排队刮削
fn process_archive(app_handle: &tauri::AppHandle, archive_path: &Path) {
    let name = archive_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("未知")
        .to_string();

    let _ = app_handle.emit(
        "import-event",
        ImportEvent::ArchiveFound {
            name: name.clone(),
            path: archive_path.to_string_lossy().to_string(),
        },
    );

    let config = crate::archive::ExtractConfig::default();
    match crate::archive::extract_archive(archive_path, &config, &|_, _, _| {}) {
        Ok(result) => {
            let _ = app_handle.emit(
                "import-event",
                ImportEvent::ArchiveExtracted {
                    name: name.clone(),
                    output: result.output_dir.clone(),
                    game_root: result.game_root.clone(),
                },
            );

            // 定位游戏根目录
            let game_dir = result
                .game_root
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(&result.output_dir));

            // 智能选择最佳 exe
            if let Some(candidate) = crate::archive::find_best_exe(&game_dir) {
                tracing::info!(
                    archive = %name,
                    exe = %candidate.path.display(),
                    score = candidate.score,
                    "Archive extracted, importing best EXE"
                );
                match import_game_smart(app_handle, &candidate.path, &game_dir) {
                    Ok(game) => {
                        // 排队自动刮削
                        let db = app_handle.state::<Database>();
                        let settings = db.get_settings();
                        if settings.auto_scrape {
                            let _ = app_handle.emit(
                                "import-event",
                                ImportEvent::ScrapeQueued {
                                    game_id: game.id.clone(),
                                    name: game.name.clone(),
                                },
                            );
                            // 触发异步刮削
                            let handle = app_handle.clone();
                            let gid = game.id.clone();
                            let gname = game.name.clone();
                            // tauri::async_runtime::spawn：监视器回调跑在 notify 自己的非 Tokio
                            // 线程上，tokio::spawn 会因无 reactor 而 panic→abort 闪退。
                            tauri::async_runtime::spawn(async move {
                                let db2 = handle.state::<Database>();
                                let s = db2.get_settings();
                                let proxy = if s.scraper_proxy.trim().is_empty() { None } else { Some(s.scraper_proxy.clone()) };
                                crate::scraper::utils::set_proxy(proxy);
                                let (raw, _) = crate::scraper::search_all(
                                    &gname,
                                    s.vndb_enabled,
                                    s.bangumi_enabled,
                                    s.dlsite_enabled,
                                    s.touchgal_enabled,
                                    s.erogamescape_enabled,
                                    s.ymgal_enabled,
                                    s.kungal_enabled,
                                    s.steam_enabled,
                                    s.pcgw_enabled,
                                )
                                .await;
                                if !raw.is_empty() {
                                    let merged = crate::scraper::merge::merge_results(
                                        raw,
                                        &crate::scraper::merge::MergeConfig {
                                            max_results: 1,
                                            ..Default::default()
                                        },
                                    );
                                    if let Some(best) = merged.first() {
                                        let cover = if let Some(ref url) = best.result.cover {
                                            Some(crate::commands::fetch_cover_to_local(url, &gid).await)
                                        } else {
                                            None
                                        };
                                        let background = if let Some(ref url) = best.result.background {
                                            Some(crate::commands::fetch_cover_to_local(url, &format!("{gid}_bg")).await)
                                        } else {
                                            None
                                        };
                                        let _ = db2.apply_scrape_result_ext(
                                            &gid,
                                            Some(best.result.title.clone()),
                                            best.result.description.clone(),
                                            cover,
                                            background,
                                            Some(best.result.tags.clone()),
                                            best.result.rating,
                                            best.result.release_year,
                                            Some(&best.result.source),
                                            Some(best.result.source_id.clone()),
                                            best.result
                                                .detail
                                                .as_ref()
                                                .and_then(|d| d.developer.clone()),
                                            best.result
                                                .detail
                                                .as_ref()
                                                .and_then(|d| d.publisher.clone()),
                                            best.result.detail.as_ref().map(|d| d.genres.clone()),
                                            best.result
                                                .detail
                                                .as_ref()
                                                .map(|d| d.languages.clone()),
                                            best.result
                                                .detail
                                                .as_ref()
                                                .and_then(|d| d.engine.clone()),
                                            best.result
                                                .detail
                                                .as_ref()
                                                .and_then(|d| d.age_rating.clone()),
                                            best.result
                                                .detail
                                                .as_ref()
                                                .and_then(|d| d.series.clone()),
                                            best.result
                                                .detail
                                                .as_ref()
                                                .and_then(|d| d.release_date.clone()),
                                            best.result
                                                .detail
                                                .as_ref()
                                                .map(|d| d.voice_languages.clone()),
                                            best.result.detail.as_ref().map(|d| d.aliases.clone()),
                                            best.result
                                                .detail
                                                .as_ref()
                                                .map(|d| d.screenshots.clone()),
                                            best.result
                                                .detail
                                                .as_ref()
                                                .and_then(|d| d.homepage.clone()),
                                        );
                                    }
                                }
                            });
                        }
                    }
                    Err(e) => {
                        let _ = app_handle.emit(
                            "import-event",
                            ImportEvent::Error {
                                message: format!("导入 {} 失败: {}", name, e),
                            },
                        );
                    }
                }
            } else {
                // 没找到 exe，扫描整个解压目录
                scan_directory(app_handle, &game_dir);
            }
        }
        Err(e) => {
            let _ = app_handle.emit(
                "import-event",
                ImportEvent::Error {
                    message: format!("解压 {} 失败: {}", name, e),
                },
            );
        }
    }
}

/// 扫描目录，使用智能 EXE 选择导入游戏
pub fn scan_directory(app_handle: &tauri::AppHandle, dir: &Path) -> (usize, usize) {
    let mut imported = 0;
    let mut skipped = 0;

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.') || name.starts_with('$') {
                    continue;
                }
            }
            if path.is_file() {
                if is_executable(&path) {
                    if is_skip_exe(&path) {
                        skipped += 1;
                    } else {
                        match import_game_smart(app_handle, &path, dir) {
                            Ok(_) => imported += 1,
                            Err(_) => skipped += 1,
                        }
                    }
                }
            } else if path.is_dir() {
                // 如果有 exe 在子目录中，优先用智能方法
                let best = crate::archive::find_best_exe(&path);
                if let Some(candidate) = best {
                    match import_game_smart(app_handle, &candidate.path, &path) {
                        Ok(_) => imported += 1,
                        Err(_) => skipped += 1,
                    }
                } else {
                    let (si, ss) = scan_directory(app_handle, &path);
                    imported += si;
                    skipped += ss;
                }
            }
        }
    }

    (imported, skipped)
}

/// 智能导入：从 exe 路径导入，使用安装目录作为 game_dir。
pub fn import_game_smart(
    app_handle: &tauri::AppHandle,
    exe_path: &Path,
    install_dir: &Path,
) -> Result<Game, String> {
    let name = exe_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("未知游戏")
        .to_string();

    let exe_path_str = exe_path.to_string_lossy().to_string();
    let install_dir_str = install_dir.to_string_lossy().to_string();

    let _ = app_handle.emit(
        "import-event",
        ImportEvent::GameFound {
            name: name.clone(),
            path: exe_path_str.clone(),
        },
    );

    let db = app_handle.state::<Database>();

    // 重复检测（M4 增强：路径 + 名称相似度）
    let existing = db.get_games();
    if crate::archive::is_duplicate(&name, &exe_path_str, &existing) {
        return Err("游戏已存在".to_string());
    }

    let mut game = Game::new(name.clone(), exe_path_str);
    game.install_dir = Some(install_dir_str);

    // 检测引擎
    if let Some(ec) = crate::locale::EngineLibrary::detect_engine(install_dir) {
        game.metadata.engine = Some(format!("{:?}", ec.engine));
    }

    let result = db.add_game(game)?;

    let _ = app_handle.emit(
        "import-event",
        ImportEvent::GameImported {
            game_id: result.id.clone(),
            name: result.name.clone(),
        },
    );

    Ok(result)
}

/// 导入已解压目录（兼容旧 API）。
fn import_game_from_path(app_handle: &tauri::AppHandle, exe_path: &Path) -> Result<Game, String> {
    let install_dir = exe_path.parent().unwrap_or_else(|| Path::new("."));
    import_game_smart(app_handle, exe_path, install_dir)
}

// ---- 文件检测辅助 ----

const ARCHIVE_EXTENSIONS: &[&str] = &["zip", "7z", "rar", "rar5", "tar", "gz", "xz"];
const EXECUTABLE_EXTENSIONS: &[&str] = &["exe", "bat", "cmd", "lnk", "msi", "com"];
const SKIP_EXE_KEYWORDS: &[&str] = &[
    "unins",
    "uninst",
    "uninstall",
    "setup",
    "install",
    "update",
    "config",
    "launcher_patch",
    "patcher",
    "patch",
    "cleanup",
    "remove",
    "vc_redist",
    "dxsetup",
    "vcredist",
    // 常见 galgame 汉化/补丁/转区工具名
    "汉化",
    "中文化",
    "chinese",
    "cn",
    "补丁",
    "修正",
    "繁体",
    "简体",
    "locale",
    "转区",
];

pub fn is_executable(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| EXECUTABLE_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn is_skip_exe(path: &Path) -> bool {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|name| {
            let lower = name.to_lowercase();
            SKIP_EXE_KEYWORDS.iter().any(|kw| lower.contains(kw))
        })
        .unwrap_or(false)
}

pub fn is_archive(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| ARCHIVE_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_is_skip_exe_chinese_locale_keywords() {
        assert!(is_skip_exe(&PathBuf::from("game_汉化.exe")));
        assert!(is_skip_exe(&PathBuf::from("patch_cn.exe")));
        assert!(is_skip_exe(&PathBuf::from("中文补丁.exe")));
        assert!(is_skip_exe(&PathBuf::from("繁体修正.exe")));
        assert!(is_skip_exe(&PathBuf::from("转区工具.exe")));
    }

    #[test]
    fn test_is_skip_exe_keeps_main_executable() {
        assert!(!is_skip_exe(&PathBuf::from("clannad.exe")));
        assert!(!is_skip_exe(&PathBuf::from("game.exe")));
        assert!(!is_skip_exe(&PathBuf::from("SiglusEngine.exe")));
    }
}
