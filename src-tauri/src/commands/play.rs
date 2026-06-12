use crate::db::Database;
use crate::locale::{self, LaunchResult, LocaleEmulatorManager};
use crate::models::{
    CompletionStatus, DailyPlaytime, Game, GamePlaytimeRank, MonthlyPlaytime, PlaySession,
    PlaySessionEntry, PlayTracker, PlaytimeSummary,
};
use crate::process_monitor::{self, ProcessMonitor};
use std::path::{Path, PathBuf};
use tauri::State;

// ===== Play tracking =====

#[tauri::command]
pub fn update_play_tracker(
    db: State<'_, Database>,
    id: String,
    tracker: PlayTracker,
) -> Result<Game, String> {
    db.update_play_tracker(&id, tracker)
}

#[tauri::command]
pub fn start_play_session(db: State<'_, Database>, id: String) -> Result<String, String> {
    db.start_play_session(&id)
}

#[tauri::command]
pub fn end_play_session(
    db: State<'_, Database>,
    id: String,
    session_id: String,
    duration_seconds: u64,
) -> Result<Game, String> {
    db.end_play_session(&id, &session_id, duration_seconds)
}

#[tauri::command]
pub fn update_completion_status(
    db: State<'_, Database>,
    id: String,
    status: CompletionStatus,
) -> Result<Game, String> {
    db.update_completion_status(&id, status)
}

#[tauri::command]
pub fn update_user_rating(
    db: State<'_, Database>,
    id: String,
    rating: Option<f64>,
) -> Result<Game, String> {
    db.update_user_rating(&id, rating)
}

#[tauri::command]
pub fn update_review(
    db: State<'_, Database>,
    id: String,
    review: Option<String>,
) -> Result<Game, String> {
    db.update_review(&id, review)
}

#[tauri::command]
pub fn update_achievements(
    db: State<'_, Database>,
    id: String,
    total: u32,
    unlocked: u32,
) -> Result<Game, String> {
    db.update_achievements(&id, total, unlocked)
}

#[tauri::command]
pub fn mark_game_finished(
    db: State<'_, Database>,
    id: String,
    finished: bool,
) -> Result<Game, String> {
    db.mark_finished(&id, finished)
}

#[tauri::command]
pub fn get_play_sessions(db: State<'_, Database>, id: String) -> Result<Vec<PlaySession>, String> {
    Ok(db.get_game(&id)?.play_tracker.sessions)
}

#[tauri::command]
pub fn update_play_session(
    db: State<'_, Database>,
    id: String,
    session_id: String,
    session: PlaySession,
) -> Result<Game, String> {
    db.update_session(&id, &session_id, session)
}

#[tauri::command]
pub fn remove_play_session(
    db: State<'_, Database>,
    id: String,
    session_id: String,
) -> Result<Game, String> {
    db.remove_session(&id, &session_id)
}

#[tauri::command]
pub fn set_play_sessions(
    db: State<'_, Database>,
    id: String,
    sessions: Vec<PlaySession>,
) -> Result<Game, String> {
    db.set_sessions(&id, sessions)
}

#[tauri::command]
pub fn update_total_playtime(
    db: State<'_, Database>,
    id: String,
    total_seconds: u64,
) -> Result<Game, String> {
    db.update_total_seconds(&id, total_seconds)
}

#[tauri::command]
pub fn update_first_played(
    db: State<'_, Database>,
    id: String,
    first_played: Option<String>,
) -> Result<Game, String> {
    db.update_first_played(&id, first_played)
}

#[tauri::command]
pub fn update_last_played(
    db: State<'_, Database>,
    id: String,
    last_played: Option<String>,
) -> Result<Game, String> {
    db.update_last_played(&id, last_played)
}

#[tauri::command]
pub fn update_completion_count(
    db: State<'_, Database>,
    id: String,
    count: u32,
) -> Result<Game, String> {
    db.update_completion_count(&id, count)
}

#[tauri::command]
pub fn get_recent_play_sessions(
    db: State<'_, Database>,
    days: Option<u32>,
    limit: Option<u32>,
) -> Vec<PlaySessionEntry> {
    let days = days.unwrap_or(30);
    let limit = limit.unwrap_or(50) as usize;
    let cutoff = chrono::Utc::now() - chrono::Duration::days(days as i64);
    let mut entries = Vec::new();

    for game in db.get_games() {
        for session in &game.play_tracker.sessions {
            if parse_session_time(&session.start_time)
                .map(|start| start >= cutoff)
                .unwrap_or(true)
            {
                entries.push(PlaySessionEntry {
                    game_id: game.id.clone(),
                    game_name: game.name.clone(),
                    session: session.clone(),
                });
            }
        }
    }

    entries.sort_by(|a, b| b.session.start_time.cmp(&a.session.start_time));
    entries.truncate(limit);
    entries
}

#[tauri::command]
pub fn get_playtime_summary(
    db: State<'_, Database>,
    days: Option<u32>,
    months: Option<u32>,
    top_limit: Option<u32>,
) -> PlaytimeSummary {
    let days = days.unwrap_or(30);
    let months = months.unwrap_or(12);
    let top_limit = top_limit.unwrap_or(10) as usize;
    let now = chrono::Utc::now();
    let day_cutoff = now - chrono::Duration::days(days as i64);
    let month_cutoff = now - chrono::Duration::days((months as i64) * 31);

    let games = db.get_games();
    let mut total_seconds = 0u64;
    let mut session_count = 0u32;
    let mut play_days = std::collections::BTreeSet::new();
    let mut daily = std::collections::BTreeMap::<String, (u64, u32)>::new();
    let mut monthly = std::collections::BTreeMap::<String, (u64, u32)>::new();
    let mut recent_sessions = Vec::new();
    let mut top_games = Vec::new();

    for game in &games {
        let game_total = game.play_tracker.total_seconds.max(game.play_time_seconds);
        total_seconds += game_total;
        session_count += game.play_tracker.sessions.len() as u32;

        top_games.push(GamePlaytimeRank {
            game_id: game.id.clone(),
            game_name: game.name.clone(),
            total_seconds: game_total,
            sessions: game.play_tracker.sessions.len() as u32,
            last_played: game.effective_last_played().map(|s| s.to_string()),
        });

        for session in &game.play_tracker.sessions {
            if let Some(date) = session.start_time.get(0..10) {
                play_days.insert(date.to_string());
            }

            let parsed = parse_session_time(&session.start_time);
            if parsed.map(|start| start >= day_cutoff).unwrap_or(true) {
                if let Some(date) = session.start_time.get(0..10) {
                    let entry = daily.entry(date.to_string()).or_insert((0, 0));
                    entry.0 += session.duration_seconds;
                    entry.1 += 1;
                }
                recent_sessions.push(PlaySessionEntry {
                    game_id: game.id.clone(),
                    game_name: game.name.clone(),
                    session: session.clone(),
                });
            }

            if parsed.map(|start| start >= month_cutoff).unwrap_or(true) {
                if let Some(month) = session.start_time.get(0..7) {
                    let entry = monthly.entry(month.to_string()).or_insert((0, 0));
                    entry.0 += session.duration_seconds;
                    entry.1 += 1;
                }
            }
        }
    }

    recent_sessions.sort_by(|a, b| b.session.start_time.cmp(&a.session.start_time));
    recent_sessions.truncate(20);

    top_games.sort_by_key(|g| std::cmp::Reverse(g.total_seconds));
    top_games.truncate(top_limit);

    PlaytimeSummary {
        total_seconds,
        session_count,
        play_days: play_days.len() as u32,
        average_session_seconds: if session_count > 0 {
            total_seconds / session_count as u64
        } else {
            0
        },
        daily: daily
            .into_iter()
            .map(|(date, (seconds, sessions))| DailyPlaytime {
                date,
                seconds,
                sessions,
            })
            .collect(),
        monthly: monthly
            .into_iter()
            .map(|(month, (seconds, sessions))| MonthlyPlaytime {
                month,
                seconds,
                sessions,
            })
            .collect(),
        recent_sessions,
        top_games,
    }
}

fn parse_session_time(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M")
        .ok()
        .map(|dt| dt.and_utc())
}

// ===== Launch =====

#[tauri::command]
pub fn launch_game(
    db: State<'_, Database>,
    lem: State<'_, LocaleEmulatorManager>,
    monitor: State<'_, ProcessMonitor>,
    app_handle: tauri::AppHandle,
    id: String,
    force_locale_jp: Option<bool>,
) -> Result<LaunchResult, String> {
    let game = db.get_game(&id)?;

    let protocol_uri = game
        .launch_uri
        .as_deref()
        .filter(|uri| is_platform_launch_uri(uri))
        .or_else(|| {
            if is_platform_launch_uri(&game.exe_path) {
                Some(game.exe_path.as_str())
            } else {
                None
            }
        });
    if let Some(uri) = protocol_uri {
        let session_id = db.start_play_session(&id)?;
        if let Err(e) = open::that(uri) {
            let _ = db.end_play_session(&id, &session_id, 0);
            return Err(format!("平台启动失败: {}", e));
        }
        let _ = db.end_play_session(&id, &session_id, 0);
        tracing::info!(
            game_id = %id,
            uri = %uri,
            "Platform game launched via URI"
        );
        return Ok(LaunchResult {
            session_id,
            engine: game.library_source.clone(),
            engine_name: game.game_type.clone(),
            locale_method: "PlatformProtocol".to_string(),
            pid: None,
        });
    }

    let install_dir = match game.install_dir.as_deref() {
        Some(d) if !d.is_empty() => PathBuf::from(d),
        _ => Path::new(&game.exe_path)
            .parent()
            .map(|p| p.to_path_buf())
            .ok_or("无法确定游戏安装目录")?,
    };

    let engine_config = locale::EngineLibrary::detect_engine(&install_dir);
    let engine_name = engine_config
        .as_ref()
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "未知引擎".into());
    let engine_variant = engine_config
        .as_ref()
        .map(|c| format!("{:?}", c.engine))
        .unwrap_or_else(|| "Other".into());

    tracing::info!(
        game_id = %id,
        engine = %engine_name,
        "Engine detected"
    );

    let exe_path = if let Some(ref config) = engine_config {
        locale::EngineLibrary::find_executable(&install_dir, config)
            .unwrap_or_else(|| PathBuf::from(&game.exe_path))
    } else {
        PathBuf::from(&game.exe_path)
    };

    if !exe_path.exists() {
        return Err(format!("可执行文件不存在: {}", exe_path.display()));
    }

    let force_jp = force_locale_jp.unwrap_or(false);
    let locale_method = if force_jp {
        if lem.is_le_available() {
            locale::LocaleMethod::LocaleEmulator
        } else if lem.is_ntleas_available() {
            locale::LocaleMethod::Ntleas
        } else {
            return Err("需要日区启动但未安装 Locale Emulator 或 NTLEAS".into());
        }
    } else if let Some(ref config) = engine_config {
        lem.get_best_method(config)
    } else {
        locale::LocaleMethod::DirectLaunch
    };

    let locale_str = if force_jp { "ja-JP" } else { "zh-CN" };
    let session_id = db.start_play_session(&id)?;
    let child = lem.launch_with_locale(&exe_path, &locale_method, locale_str)?;
    let child_id = child.id();

    monitor.register(child_id, &session_id, &id);
    process_monitor::spawn_exit_watcher(child, app_handle.clone());

    if game.metadata.engine.is_none() {
        let _ = db.update_engine(&id, Some(engine_variant.clone()));
    }

    if exe_path.to_string_lossy() != game.exe_path {
        let _ = db.update_exe_path(&id, exe_path.to_string_lossy().to_string());
    }

    let result = LaunchResult {
        session_id,
        engine: Some(engine_variant),
        engine_name: Some(engine_name),
        locale_method: format!("{:?}", locale_method),
        pid: Some(child_id),
    };

    tracing::info!(
        game_id = %id,
        session_id = %result.session_id,
        pid = child_id,
        engine = %result.engine_name.as_deref().unwrap_or("?"),
        method = %result.locale_method,
        "Game launched"
    );

    Ok(result)
}

pub(crate) fn is_platform_launch_uri(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    lower.starts_with("steam://")
        || lower.starts_with("com.epicgames.launcher://")
        || lower.starts_with("uplay://")
        || lower.starts_with("origin://")
        || lower.starts_with("goggalaxy://")
}
