// 萌游 MoeGame - 库入口
#![allow(clippy::field_reassign_with_default, clippy::too_many_arguments)]

pub mod anime;
pub mod comic;
pub mod archive;
pub mod auto_scrape;
pub mod autostart;
pub mod cloud_save;
pub mod commands;
pub mod csharp_migration;
pub mod db;
pub mod db_sqlite;
pub mod diagnostics;
pub mod downloader;
pub mod emulator;
pub mod gal_download;
pub mod http_client;
pub mod image_scanner;
pub mod import;
pub mod integration;
pub mod locale;
pub mod logging;
pub mod migration;
pub mod models;
pub mod nsfw;
pub mod performance;
pub mod process_monitor;
pub mod recommender;
pub mod resource_fetcher;
pub mod scraper;
pub mod security;
pub mod stats;
pub mod steam_openid;
pub mod sync;
pub mod task_queue;
pub mod thumbnail;
pub mod translator;
pub mod utils;

pub mod video_extractor;
pub mod video_proxy;
pub mod external_player;
pub mod anime_download;
use db::Database;
use downloader::Downloader;
use anime_download::AnimeDownloader;
use import::ImportWatcher;
use locale::LocaleEmulatorManager;
use process_monitor::ProcessMonitor;
use std::path::PathBuf;
use task_queue::TaskQueue;
use tauri::Manager;

/// 启动 Tauri 应用（桌面入口）
pub fn crash_log(msg: &str) {
    use std::io::Write;
    let dir = dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("moeplay").join("logs");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("crash.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let ts = chrono::Local::now().format("%H:%M:%S%.3f");
        let _ = writeln!(f, "[{}] {}", ts, msg);
        let _ = f.flush();
    }
}

pub fn run() {
    crash_log("run() START");
    // 初始化结构化日志
    logging::init();
    crash_log("logging::init() done");

    tracing::info!("=== MoeGame v{} starting ===", env!("CARGO_PKG_VERSION"));

    // 启动时清理过期缓存（同步）
    let pruned = scraper::global_cache().prune();
    if pruned > 0 {
        tracing::info!(pruned, "Cleaned expired scrape cache entries");
    }

    // 默认下载目录
    let download_dir = dirs::download_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("萌游下载");

    // 番剧下载目录
    let anime_download_dir = dirs::download_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("萌游下载")
        .join("番剧");

    crash_log("Building Tauri app...");
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(Database::new())
        .manage(anime::AnimeState::default())
        .manage(comic::ComicState::default())
        .manage(Downloader::new(download_dir, 3))
        .manage(AnimeDownloader::new(anime_download_dir))
        .manage(TaskQueue::new())
        .manage(LocaleEmulatorManager::new())
        .manage(ProcessMonitor::new())
        .manage(ImportWatcher::new())
        .invoke_handler(tauri::generate_handler![
            // ---- 游戏查询 ----
            commands::get_games,
            commands::get_game,
            commands::search_games,
            // ---- 游戏增删改 ----
            commands::add_game_by_path,
            commands::add_game_by_dialog,
            commands::delete_game,
            commands::update_game,
            commands::import_games_from_dir,
            // ---- M4 自动化 ----
            commands::scan_directory_for_games,
            commands::preview_directory_for_games,
            commands::import_selected_candidates,
            commands::extract_archive_command,
            commands::start_import_watcher_cmd,
            commands::stop_import_watcher_cmd,
            commands::get_import_watcher_status,
            // ---- 基本信息更新 ----
            commands::update_game_name,
            commands::update_game_description,
            commands::update_game_cover,
            commands::update_game_background,
            commands::update_game_icon,
            commands::update_game_type,
            commands::update_install_dir,
            commands::update_exe_path,
            // ---- 快捷切换 ----
            commands::toggle_favorite,
            commands::toggle_hidden,
            // ---- 简单标签 ----
            commands::add_simple_tag,
            commands::remove_simple_tag,
            commands::set_simple_tags,
            // ---- 增强标签 ----
            commands::add_tag_entry,
            commands::remove_tag_entry,
            commands::update_tag_entry,
            commands::set_tag_entries,
            // ---- 别名 ----
            commands::add_game_alias,
            commands::remove_game_alias,
            commands::set_primary_alias,
            commands::set_game_aliases,
            // ---- 元数据 ----
            commands::update_game_metadata,
            commands::update_developer,
            commands::update_publisher,
            commands::update_platform,
            commands::update_engine,
            commands::update_game_version,
            commands::update_original_name,
            commands::update_homepage,
            commands::update_developer_homepage,
            commands::update_age_rating,
            commands::update_series,
            commands::update_release_date,
            commands::update_release_year,
            commands::update_estimated_hours,
            commands::update_vndb_rating,
            commands::update_bangumi_rating,
            commands::update_vndb_id,
            commands::update_bangumi_id,
            commands::set_genres,
            commands::set_languages,
            commands::set_voice_languages,
            // ---- 游玩追踪 ----
            commands::update_play_tracker,
            commands::start_play_session,
            commands::end_play_session,
            commands::update_completion_status,
            commands::update_user_rating,
            commands::update_review,
            commands::update_achievements,
            commands::mark_game_finished,
            commands::get_play_sessions,
            commands::update_play_session,
            commands::remove_play_session,
            commands::set_play_sessions,
            commands::update_total_playtime,
            commands::update_first_played,
            commands::update_last_played,
            commands::update_completion_count,
            commands::get_recent_play_sessions,
            commands::get_playtime_summary,
            // ---- 截图 ----
            commands::add_screenshot,
            commands::remove_screenshot,
            commands::remove_screenshot_by_path,
            commands::set_screenshots,
            // ---- 存档数据 ----
            commands::update_save_data,
            commands::set_save_dir,
            commands::configure_auto_backup,
            commands::add_game_backup,
            commands::remove_game_backup,
            commands::update_backup_note,
            commands::configure_cloud_sync,
            // ---- 启动 ----
            commands::launch_game,
            // ---- M2 引擎/区域 ----
            commands::detect_game_engine,
            commands::get_locale_emulator_status,
            commands::set_custom_le_path,
            commands::get_running_games,
            // ---- 刮削 ----
            commands::scrape_games,
            commands::scrape_game,
            commands::scrape_dlsite_product,
            commands::scrape_erogamescape_game,
            commands::scrape_ymgal_detail,
            commands::scrape_kungal_detail,
            commands::scrape_steam_app,
            commands::scrape_pcgw_page,
            commands::apply_scrape_result,
            commands::fetch_vndb_detail,
            commands::fetch_bangumi_detail,
            commands::fetch_full_detail,
            // ---- M3 刮削增强 ----
            commands::scrape_game_merged,
            commands::get_ai_providers,
            commands::get_ai_presets,
            commands::run_ai_preset,
            commands::download_screenshots,
            // ---- 存档（文件系统扫描） ----
            commands::get_game_saves,
            commands::backup_save,
            commands::restore_save,
            commands::detect_save_candidates,
            commands::scan_save_dir,
            commands::create_save_snapshot,
            commands::list_save_snapshots,
            commands::restore_save_snapshot,
            commands::delete_save_snapshot,
            commands::compare_save_snapshot,
            commands::detect_save_conflicts,
            commands::sync_save_snapshots_to_cloud,
            commands::restore_latest_save_snapshot_from_cloud,
            // ---- NSFW / 翻译 ----
            commands::get_nsfw_decision,
            commands::classify_nsfw_game,
            commands::get_games_nsfw_filtered,
            commands::update_nsfw_display_mode,
            commands::translate_scrape_metadata,
            commands::translate_text,
            commands::parse_chinese_metadata,
            commands::embed_chinese_metadata,
            commands::strip_metadata_markers,
            commands::parse_scrape_marker,
            commands::embed_scrape_marker,
            // ---- 设置 ----
            commands::get_settings,
            commands::update_settings,
            commands::add_watch_dir,
            commands::remove_watch_dir,
            commands::pick_directory,
            commands::pick_image_file,
            // ---- 数据库信息 ----
            commands::get_schema_version,
            commands::get_game_count,
            // ---- P1 增强体验 ----
            commands::get_recommendations,
            commands::get_dashboard_data,
            commands::get_smart_collections,
            commands::get_collection_games,
            commands::cache_thumbnail,
            commands::get_thumbnail,
            commands::clear_thumbnail_cache,
            commands::enqueue_task,
            commands::get_tasks,
            commands::update_task,
            commands::cancel_task,
            commands::clear_finished_tasks,
            commands::get_migration_status,
            commands::export_database,
            commands::import_database,
            commands::scan_images_dir,
            commands::scan_game_images,
            commands::get_performance_snapshot,
            commands::run_diagnostics,
            // ---- 下载管理 ----
            commands::download_start,
            commands::download_pause,
            commands::download_resume,
            commands::download_cancel,
            commands::download_cancel_all,
            commands::download_retry,
            commands::download_remove,
            commands::download_clear_finished,
            commands::get_downloads,
            commands::set_download_speed_limit,
            commands::get_download_speed_limit,
            commands::set_download_max_concurrent,
            commands::get_download_max_concurrent,
            // ---- M1 C# 迁移桥 ----
            commands::migrate_from_csharp,
            commands::verify_migration,
            commands::verify_migration_ids,
            // ---- 工具 ----
            commands::open_url,
            commands::open_path,
            commands::fetch_game_resources,
            commands::search_game_downloads,
            commands::search_downloads_direct,
            // ---- M6 Steam 集成 ----
            commands::find_steam_path,
            commands::scan_steam_library,
            commands::scan_epic_library,
            commands::import_steam_game,
            commands::get_platform_import_status,
            commands::resolve_steam_id,
            commands::validate_steam_api_key,
            commands::steam_login_openid,
            commands::scan_platform_library,
            commands::import_platform_library,
            commands::import_steam_session_games,
            commands::sync_steam_achievements,
            // ---- M6 云存档 + 诊断 ----
            commands::backup_snapshot_local,
            commands::test_webdav_connection,
            commands::export_diagnostics_zip,
            // ---- M6 自动入库刮削 ----
            commands::run_auto_scrape_pipeline,
            // ---- M6 Steam 身份认证 + Web API ----
            commands::steam_open_community,
            commands::steam_login_webview,
            commands::steam_resolve_url,
            commands::steam_openid_login,
            commands::steam_verify_api_key,
            commands::steam_detect_local,
            commands::steam_fetch_owned_games,
            commands::steam_fetch_and_import,
            commands::steam_import_owned_games,
            // ---- 模拟器检测与 ROM 导入 ----
            commands::search_emulators,
            commands::scan_roms,
            commands::import_rom_game,
            // ---- 开机自启 ----
            commands::set_autostart,
            commands::get_autostart_status,
            // ---- 哔咔漫画 ----
            commands::comic_set_token,
            commands::comic_login,
            commands::comic_profile,
            commands::comic_categories,
            commands::comic_list,
            commands::comic_search,
            commands::comic_detail,
            commands::comic_chapters,
            commands::comic_chapter_images,
            commands::comic_ranking,
            commands::comic_random,
            commands::comic_favorites,
            commands::comic_toggle_favourite,
            commands::comic_like,
            commands::comic_comments,
            commands::comic_post_comment,
            commands::comic_comment_like,
            commands::comic_comment_children,
            commands::comic_recommendation,
            commands::comic_punch_in,
            commands::comic_knight_leaderboard,
            commands::comic_my_comments,
            // ---- 番剧规则引擎 ----
            commands::anime_get_rules,
            commands::anime_set_rules,
            commands::anime_add_rule,
            commands::anime_remove_rule,
            commands::anime_import_rules,
            commands::anime_search,
            commands::anime_search_all,
            commands::anime_fetch_roads,
            commands::anime_build_url,
            commands::anime_fetch_page,
            // ---- 番剧 GitHub 规则仓库 + Bangumi ----
            commands::anime_github_rules_index,
            commands::anime_install_github_rule,
            commands::anime_install_all_github_rules,
            commands::anime_bangumi_calendar,
            commands::anime_bangumi_search,
            commands::anime_proxy_image,
            commands::anime_proxy_images_batch,
            // ---- Bangumi 详情 ----
            commands::anime_bangumi_detail,
            commands::anime_bangumi_rating,
            commands::anime_bangumi_characters,
            commands::anime_bangumi_persons,
            commands::anime_bangumi_comments,
            commands::anime_bangumi_episodes_list,
            commands::anime_get_proxy_url,
            commands::frontend_log,
            commands::anime_image_search,
            commands::anime_bangumi_episode_comments,
            // ---- Bangumi 收藏同步 ----
            commands::anime_bangumi_get_username,
            commands::anime_bangumi_get_user_collection,
            commands::anime_bangumi_get_all_collections,
            commands::anime_bangumi_update_collection,
            // ---- 视频提取 ----
            video_extractor::extract_video_url,
            video_extractor::anime_extract_video_url,
            // ---- DanDanPlay 弹幕 ----
            commands::anime_danmaku_search,
            commands::anime_danmaku_get_episodes,
            commands::anime_danmaku_get_comments,
            // ---- 外部播放器 ----
            commands::anime_get_external_players,
            commands::anime_launch_external_player,
            // ---- 番剧下载 ----
            commands::anime_download_episode,
            commands::anime_get_downloads,
            commands::anime_cancel_download,
            commands::anime_pause_download,
            commands::anime_resume_download,
            commands::anime_remove_download,
            commands::anime_clear_finished_downloads,
            commands::anime_open_download_folder,
        ])
        .setup(|app| {
            crash_log("setup() ENTER");
            // 启动时按持久化的 startup_mode 原生设定窗口模式。
            // 直接 set_fullscreen(true) 在 Windows 上可能因窗口尚未完全初始化而静默失败。
            // 因此：先在 setup 内试一次（大多数情况有效），再延迟 200ms 重试一次（兜底）。
            // 启动窗口模式：不在 .setup() 里读数据库/调全屏——
            //   Tauri setup 阶段 WebView2 状态不稳定，任何非平凡操作都可能触发
            //   栈溢出 (0xc0000409)。全屏交给前端 $effect 安全处理。
            if app.get_webview_window("main").is_some() {
                crash_log("setup() main window ready — fullscreen via config, mode via frontend");
            }

            // 仅清理超过 30 天未更新的缩略图，而不是每次启动全清。
            // 之前调用 clear_thumbnail_cache() 会清空整盘缓存，导致 500+ 封面每次启动全部重生成、首屏变慢。
            tauri::async_runtime::spawn(async {
                let _ = thumbnail::prune_thumbnails(30);
            });

            // 启动视频流代理服务器（解决 CORS / 防盗链 Referer 问题）
            video_proxy::start_proxy_server(app.handle().clone());
            crash_log("setup() DONE");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    crash_log("run() EXIT — this should never be reached!");
}
