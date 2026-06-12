// 统计模块 - 仪表盘、游玩统计、智能合集
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 递归计算目录大小（字节）
fn dir_size(path: &PathBuf) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += dir_size(&p);
            } else if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

use crate::db::Database;
use crate::models::*;
use crate::nsfw;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthActivity {
    pub month: String,
    pub sessions: u32,
    pub hours: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    let total = games.len() as u32;

    let installed = games.iter().filter(|g| g.install_dir.is_some()).count() as u32;
    let completed = games
        .iter()
        .filter(|g| g.play_tracker.completion_status == CompletionStatus::Completed)
        .count() as u32;
    let playtime: f64 = games
        .iter()
        .map(|g| g.play_tracker.total_seconds as f64)
        .sum::<f64>()
        / 3600.0;
    let completion_rate = if total > 0 {
        completed as f64 / total as f64 * 100.0
    } else {
        0.0
    };

    // 刮削覆盖率
    let scraped = games.iter().filter(|g| g.description.is_some()).count() as u32;
    let scrape_coverage = if total > 0 {
        scraped as f64 / total as f64 * 100.0
    } else {
        0.0
    };

    // 最近游玩
    let mut recent: Vec<_> = games
        .iter()
        .filter(|g| g.effective_last_played().is_some())
        .collect();
    recent.sort_by(|a, b| b.effective_last_played().cmp(&a.effective_last_played()));
    let recent_games: Vec<String> = recent.iter().take(5).map(|g| g.name.clone()).collect();

    // 标签统计
    let mut tag_count: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for game in &games {
        for tag in &game.tags {
            *tag_count.entry(tag.clone()).or_insert(0) += 1;
        }
    }
    let mut top_tags: Vec<_> = tag_count.into_iter().collect();
    top_tags.sort_by_key(|tag| std::cmp::Reverse(tag.1));
    let top_tags = top_tags.into_iter().take(10).collect();

    // 通关状态分布
    let status_count = games
        .iter()
        .fold(std::collections::HashMap::new(), |mut map, g| {
            let status = format!("{:?}", g.play_tracker.completion_status);
            *map.entry(status).or_insert(0) += 1u32;
            map
        });
    let completion_distribution: Vec<_> = status_count.into_iter().collect();

    // 智能合集
    let collections = generate_collections(&games);

    // 月度热力图
    let monthly_heatmap = build_monthly_heatmap(&games);

    // 磁盘占用：遍历安装目录计算实际大小
    let disk_usage_gb = games
        .iter()
        .filter_map(|g| g.install_dir.as_ref())
        .map(|dir| dir_size(&PathBuf::from(dir)))
        .sum::<u64>() as f64
        / 1_073_741_824.0;

    DashboardData {
        total_games: total,
        installed_games: installed,
        completed_games: completed,
        playtime_hours: (playtime * 10.0).round() / 10.0,
        completion_rate: (completion_rate * 10.0).round() / 10.0,
        scrape_coverage: (scrape_coverage * 10.0).round() / 10.0,
        disk_usage_gb: (disk_usage_gb * 10.0).round() / 10.0,
        recent_games,
        top_tags,
        completion_distribution,
        monthly_heatmap,
        collections,
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
                        chrono::NaiveDateTime::parse_from_str(&g.created_at, "%Y-%m-%d %H:%M")
                            .map(|created| {
                                let created = created.and_utc();
                                let now = chrono::Utc::now();
                                (now - created).num_days() < 30
                            })
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

/// 构建最近12个月月度游玩热力图
fn build_monthly_heatmap(games: &[Game]) -> Vec<MonthActivity> {
    let now = chrono::Utc::now();
    let mut months: std::collections::BTreeMap<String, (u32, f64)> =
        std::collections::BTreeMap::new();
    for i in 0..12 {
        let m = now - chrono::Duration::days(i * 30);
        let key = m.format("%Y-%m").to_string();
        months.insert(key, (0, 0.0));
    }
    for game in games {
        for session in &game.play_tracker.sessions {
            if let Some(ref end) = session.end_time {
                if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(end, "%Y-%m-%d %H:%M:%S") {
                    let key = dt.format("%Y-%m").to_string();
                    if let Some(entry) = months.get_mut(&key) {
                        entry.0 += 1;
                        entry.1 += session.duration_seconds as f64 / 3600.0;
                    }
                }
            }
        }
        if let Some(last) = game.effective_last_played() {
            if let Some(dt) = parse_play_time(last) {
                let key = dt.format("%Y-%m").to_string();
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
            hours: (hours * 10.0).round() / 10.0,
        })
        .collect()
}

fn parse_play_time(value: &str) -> Option<chrono::NaiveDateTime> {
    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M"))
        .ok()
}

pub fn filter_collection_games(games: &[Game], collection_id: &str) -> Vec<Game> {
    let mut result = games
        .iter()
        .filter(|game| match collection_id {
            "not_played" => game.play_tracker.completion_status == CompletionStatus::NotStarted,
            "recent" => chrono::NaiveDateTime::parse_from_str(&game.created_at, "%Y-%m-%d %H:%M")
                .map(|created| (chrono::Utc::now() - created.and_utc()).num_days() < 30)
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
