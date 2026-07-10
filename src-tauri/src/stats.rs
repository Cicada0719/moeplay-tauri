// 统计模块 - 仪表盘、游玩统计、智能合集
use chrono::{DateTime, Datelike, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::db::Database;
use crate::models::*;
use crate::nsfw;

const DISK_USAGE_CACHE_TTL: Duration = Duration::from_secs(5 * 60);
const GIB_BYTES: f64 = 1_073_741_824.0;

#[derive(Default)]
struct DiskUsageCache {
    install_dirs: Vec<PathBuf>,
    bytes: u64,
    updated_at: Option<Instant>,
    scanning: bool,
}

static DISK_USAGE_CACHE: OnceLock<Mutex<DiskUsageCache>> = OnceLock::new();

fn disk_usage_cache() -> &'static Mutex<DiskUsageCache> {
    DISK_USAGE_CACHE.get_or_init(|| Mutex::new(DiskUsageCache::default()))
}

/// 递归计算目录大小（字节）。此函数只允许在后台磁盘扫描线程调用。
fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    let mut pending = vec![path.to_path_buf()];

    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                pending.push(entry.path());
            } else if file_type.is_file() {
                if let Ok(meta) = entry.metadata() {
                    total = total.saturating_add(meta.len());
                }
            }
        }
    }

    total
}

fn install_dirs(games: &[Game]) -> Vec<PathBuf> {
    let mut dirs: Vec<_> = games
        .iter()
        .filter_map(|game| game.install_dir.as_deref())
        .map(PathBuf::from)
        .collect();
    dirs.sort();
    dirs.dedup();
    dirs
}

/// 返回最近一次缓存值，并在缓存过期时启动后台扫描。
///
/// Dashboard 首屏不会等待递归 I/O。后续需要由 Integrator 增加独立的异步
/// `refresh_dashboard_disk_usage` command/event，让前端在扫描完成后主动刷新；在该接口
/// 注册前，重新进入页面或手动重试会读取已完成的缓存。
fn cached_disk_usage_gb(games: &[Game]) -> f64 {
    let dirs = install_dirs(games);
    if dirs.is_empty() {
        return 0.0;
    }

    let now = Instant::now();
    let mut cache = disk_usage_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let cache_matches = cache.install_dirs == dirs;
    let cache_fresh = cache_matches
        && cache
            .updated_at
            .is_some_and(|updated| now.duration_since(updated) < DISK_USAGE_CACHE_TTL);
    let cached_bytes = if cache_matches { cache.bytes } else { 0 };

    if !cache_fresh && !cache.scanning {
        cache.scanning = true;
        let scan_dirs = dirs;
        drop(cache);

        let spawn_result = std::thread::Builder::new()
            .name("dashboard-disk-scan".to_string())
            .spawn(move || {
                let bytes = scan_dirs
                    .iter()
                    .map(|path| dir_size(path))
                    .fold(0u64, u64::saturating_add);
                let mut cache = disk_usage_cache()
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                cache.install_dirs = scan_dirs;
                cache.bytes = bytes;
                cache.updated_at = Some(Instant::now());
                cache.scanning = false;
            });

        if spawn_result.is_err() {
            let mut cache = disk_usage_cache()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            cache.scanning = false;
        }
    }

    cached_bytes as f64 / GIB_BYTES
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DashboardData {
    pub total_games: u32,
    pub installed_games: u32,
    pub completed_games: u32,
    pub playtime_hours: f64,
    pub completion_rate: f64,
    pub scrape_coverage: f64,
    pub disk_usage_gb: f64,
    pub recent_games: Vec<String>,
    pub top_tags: Vec<(String, u32)>,
    pub completion_distribution: Vec<(String, u32)>,
    pub monthly_heatmap: Vec<MonthActivity>,
    pub collections: Vec<Collection>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonthActivity {
    pub month: String,
    pub sessions: u32,
    pub hours: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub description: String,
    pub game_count: u32,
    pub icon: String,
}

/// 生成仪表盘数据
pub fn generate_dashboard(db: &Database) -> DashboardData {
    let games = db.get_games();
    generate_dashboard_from_games(&games)
}

/// 从游戏快照生成仪表盘数据，供命令层和契约测试复用。
pub fn generate_dashboard_from_games(games: &[Game]) -> DashboardData {
    let total = games.len() as u32;
    let installed = games
        .iter()
        .filter(|game| game.install_dir.is_some())
        .count() as u32;
    let completed = games
        .iter()
        .filter(|game| game.play_tracker.completion_status == CompletionStatus::Completed)
        .count() as u32;
    let playtime = games
        .iter()
        .map(|game| game.play_tracker.total_seconds.max(game.play_time_seconds) as f64)
        .sum::<f64>()
        / 3600.0;
    let completion_rate = percentage(completed, total);

    let scraped = games
        .iter()
        .filter(|game| game.description.is_some())
        .count() as u32;
    let scrape_coverage = percentage(scraped, total);

    let mut recent: Vec<_> = games
        .iter()
        .filter(|game| game.effective_last_played().is_some())
        .collect();
    recent.sort_by(|a, b| {
        let a_time = a.effective_last_played().and_then(parse_play_time);
        let b_time = b.effective_last_played().and_then(parse_play_time);
        b_time
            .cmp(&a_time)
            .then_with(|| b.effective_last_played().cmp(&a.effective_last_played()))
    });
    let recent_games = recent
        .into_iter()
        .take(5)
        .map(|game| game.name.clone())
        .collect();

    let mut tag_count: HashMap<String, u32> = HashMap::new();
    for game in games {
        for tag in &game.tags {
            *tag_count.entry(tag.clone()).or_insert(0) += 1;
        }
    }
    let mut top_tags: Vec<_> = tag_count.into_iter().collect();
    top_tags.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    top_tags.truncate(10);

    let mut status_count: BTreeMap<String, u32> = BTreeMap::new();
    for game in games {
        *status_count
            .entry(completion_status_key(&game.play_tracker.completion_status).to_string())
            .or_insert(0) += 1;
    }

    DashboardData {
        total_games: total,
        installed_games: installed,
        completed_games: completed,
        playtime_hours: round_one_decimal(playtime),
        completion_rate: round_one_decimal(completion_rate),
        scrape_coverage: round_one_decimal(scrape_coverage),
        disk_usage_gb: round_one_decimal(cached_disk_usage_gb(games)),
        recent_games,
        top_tags,
        completion_distribution: status_count.into_iter().collect(),
        monthly_heatmap: build_monthly_heatmap(games),
        collections: generate_collections(games),
    }
}

fn percentage(value: u32, total: u32) -> f64 {
    if total == 0 {
        0.0
    } else {
        value as f64 / total as f64 * 100.0
    }
}

fn round_one_decimal(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn completion_status_key(status: &CompletionStatus) -> &'static str {
    match status {
        CompletionStatus::NotStarted => "not_started",
        CompletionStatus::Playing => "playing",
        CompletionStatus::Completed => "completed",
        CompletionStatus::Dropped => "dropped",
        CompletionStatus::OnHold => "on_hold",
        CompletionStatus::PlanToPlay => "plan_to_play",
        CompletionStatus::Replaying => "replaying",
    }
}

/// 生成智能合集 (10类规则)
pub fn generate_collections(games: &[Game]) -> Vec<Collection> {
    let collections = vec![
        ("not_played", "未玩过", "尚未开始游玩的游戏", "play"),
        ("recent", "最近添加", "最近入库的游戏", "plus"),
        ("completed", "已通关", "已通关的游戏", "check"),
        ("high_rated", "高分游戏", "评分 >= 8 的游戏", "star"),
        ("unscraped", "未刮削", "尚未获取元数据的游戏", "search"),
        ("favorites", "收藏夹", "收藏的游戏", "heart"),
        ("by_dev", "按开发商", "相同开发商的游戏", "toolbox"),
        ("by_tag", "按标签", "相同标签的游戏", "tag"),
        ("by_year", "按年份", "相同年份的游戏", "chart"),
        ("nsfw", "成人内容", "NSFW 标记的游戏", "x"),
    ];

    collections
        .iter()
        .map(|(id, name, desc, icon)| {
            let count = match *id {
                "not_played" => games
                    .iter()
                    .filter(|g| g.play_tracker.completion_status == CompletionStatus::NotStarted)
                    .count(),
                "recent" => games
                    .iter()
                    .filter(|g| {
                        parse_play_time(&g.created_at)
                            .map(|created| (Utc::now() - created).num_days() < 30)
                            .unwrap_or(false)
                    })
                    .count(),
                "completed" => games
                    .iter()
                    .filter(|g| g.play_tracker.completion_status == CompletionStatus::Completed)
                    .count(),
                "high_rated" => games
                    .iter()
                    .filter(|g| g.effective_rating().unwrap_or(0.0) >= 8.0)
                    .count(),
                "unscraped" => games.iter().filter(|g| g.description.is_none()).count(),
                "favorites" => games.iter().filter(|g| g.favorite).count(),
                "by_dev" => games
                    .iter()
                    .filter(|g| g.metadata.developer.is_some())
                    .count(),
                "by_tag" => games
                    .iter()
                    .filter(|g| !g.tags.is_empty() || !g.tag_entries.is_empty())
                    .count(),
                "by_year" => games
                    .iter()
                    .filter(|g| g.effective_release_year().is_some())
                    .count(),
                "nsfw" => games.iter().filter(|g| nsfw::is_nsfw(g)).count(),
                _ => 0,
            };
            Collection {
                id: id.to_string(),
                name: name.to_string(),
                description: desc.to_string(),
                game_count: count as u32,
                icon: icon.to_string(),
            }
        })
        .collect()
}

/// 构建最近12个月月度游玩热力图。
pub fn build_monthly_heatmap(games: &[Game]) -> Vec<MonthActivity> {
    build_monthly_heatmap_at(games, Utc::now())
}

/// 固定当前时间的聚合入口，便于覆盖跨月、时区和 legacy fixture。
#[doc(hidden)]
pub fn build_monthly_heatmap_at(games: &[Game], now: DateTime<Utc>) -> Vec<MonthActivity> {
    let mut months: BTreeMap<String, (u32, f64)> = BTreeMap::new();
    for months_ago in (0..12).rev() {
        months.insert(month_key(now, months_ago), (0, 0.0));
    }

    let mut seen_sessions = HashSet::new();
    for game in games {
        for session in &game.play_tracker.sessions {
            let Some(end_time) = session.end_time.as_deref() else {
                continue;
            };
            let identity = session_identity(game, session);
            if !seen_sessions.insert(identity) {
                continue;
            }
            let Some(timestamp) = parse_play_time(end_time) else {
                continue;
            };
            let key = timestamp.format("%Y-%m").to_string();
            if let Some(entry) = months.get_mut(&key) {
                entry.0 += 1;
                entry.1 += session.duration_seconds as f64 / 3600.0;
            }
        }

        // 旧库可能只有 last_played 而没有 session 历史。只有这种情况才补一个
        // 兼容计数，避免同一 session 又被 last_played 重复累计。
        if game.play_tracker.sessions.is_empty() {
            if let Some(timestamp) = game.effective_last_played().and_then(parse_play_time) {
                let key = timestamp.format("%Y-%m").to_string();
                if let Some(entry) = months.get_mut(&key) {
                    entry.0 += 1;
                }
            }
        }
    }

    months
        .into_iter()
        .map(|(month, (sessions, hours))| MonthActivity {
            month,
            sessions,
            hours: round_one_decimal(hours),
        })
        .collect()
}

fn month_key(now: DateTime<Utc>, months_ago: i32) -> String {
    let absolute_month = now.year() * 12 + now.month0() as i32 - months_ago;
    let year = absolute_month.div_euclid(12);
    let month = absolute_month.rem_euclid(12) + 1;
    format!("{year:04}-{month:02}")
}

fn session_identity(game: &Game, session: &PlaySession) -> String {
    if !session.id.trim().is_empty() {
        format!("id:{}", session.id)
    } else {
        format!(
            "legacy:{}:{}:{}:{}",
            game.id,
            session.start_time,
            session.end_time.as_deref().unwrap_or_default(),
            session.duration_seconds
        )
    }
}

/// 时间读取优先采用 RFC3339（含偏移与毫秒），随后兼容旧版无时区格式。
fn parse_play_time(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .or_else(|_| NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S").map(|v| v.and_utc()))
        .or_else(|_| NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M").map(|v| v.and_utc()))
        .ok()
}

pub fn filter_collection_games(games: &[Game], collection_id: &str) -> Vec<Game> {
    let mut result = games
        .iter()
        .filter(|game| match collection_id {
            "not_played" => game.play_tracker.completion_status == CompletionStatus::NotStarted,
            "recent" => parse_play_time(&game.created_at)
                .map(|created| (Utc::now() - created).num_days() < 30)
                .unwrap_or(false),
            "completed" => game.play_tracker.completion_status == CompletionStatus::Completed,
            "high_rated" => game.effective_rating().unwrap_or(0.0) >= 8.0,
            "unscraped" => game.description.is_none(),
            "favorites" => game.favorite,
            "by_dev" => game.metadata.developer.is_some(),
            "by_tag" => !game.tags.is_empty() || !game.tag_entries.is_empty(),
            "by_year" => game.effective_release_year().is_some(),
            "nsfw" => nsfw::is_nsfw(game),
            _ => true,
        })
        .cloned()
        .collect::<Vec<_>>();
    result.sort_by(|a, b| a.name.cmp(&b.name));
    result
}
