//! AI/规则推荐引擎
//!
//! 不依赖外部模型时，基于标签、开发商、年份、评分、完成状态做可解释推荐。

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::models::{CompletionStatus, Game};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub game_id: String,
    pub name: String,
    pub score: f64,
    pub reasons: Vec<String>,
}

pub fn recommend_games(
    games: &[Game],
    seed_game_id: Option<&str>,
    limit: usize,
) -> Vec<Recommendation> {
    let seed = seed_game_id.and_then(|id| games.iter().find(|game| game.id == id));
    let favorite_tags = favorite_tags(games);
    let favorite_developers = favorite_developers(games);

    let mut recommendations = games
        .iter()
        .filter(|game| seed.map(|s| s.id != game.id).unwrap_or(true))
        .filter(|game| game.play_tracker.completion_status != CompletionStatus::Completed)
        .map(|game| score_game(game, seed, &favorite_tags, &favorite_developers))
        .filter(|rec| rec.score > 0.0)
        .collect::<Vec<_>>();

    recommendations.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    recommendations.truncate(limit);
    recommendations
}

fn score_game(
    game: &Game,
    seed: Option<&Game>,
    favorite_tags: &HashMap<String, u32>,
    favorite_developers: &HashSet<String>,
) -> Recommendation {
    let mut score = 0.0;
    let mut reasons = Vec::new();

    if game.favorite {
        score += 18.0;
        reasons.push("已收藏".to_string());
    }

    if let Some(rating) = game.effective_rating() {
        if rating >= 8.0 {
            score += (rating - 7.0) * 8.0;
            reasons.push(format!("高评分 {:.1}", rating));
        }
    }

    for tag in game.tags.iter().chain(game.metadata.genres.iter()) {
        let key = tag.to_lowercase();
        if let Some(count) = favorite_tags.get(&key) {
            score += (*count as f64).min(5.0) * 4.0;
            reasons.push(format!("偏好标签 {}", tag));
        }
    }

    if let Some(ref dev) = game.metadata.developer {
        if favorite_developers.contains(&dev.to_lowercase()) {
            score += 14.0;
            reasons.push(format!("偏好开发商 {}", dev));
        }
    }

    if let Some(seed) = seed {
        let shared = shared_tags(seed, game);
        if !shared.is_empty() {
            score += (shared.len() as f64) * 10.0;
            reasons.push(format!("与 {} 共享标签: {}", seed.name, shared.join(", ")));
        }
        if seed.metadata.developer.is_some() && seed.metadata.developer == game.metadata.developer {
            score += 18.0;
            reasons.push("同开发商".to_string());
        }
        if seed.effective_release_year().is_some()
            && seed.effective_release_year() == game.effective_release_year()
        {
            score += 6.0;
            reasons.push("同发行年份".to_string());
        }
    }

    if game.play_tracker.completion_status == CompletionStatus::NotStarted {
        score += 5.0;
        reasons.push("尚未开始".to_string());
    }

    Recommendation {
        game_id: game.id.clone(),
        name: game.name.clone(),
        score: (score * 10.0).round() / 10.0,
        reasons,
    }
}

fn favorite_tags(games: &[Game]) -> HashMap<String, u32> {
    let mut tags = HashMap::new();
    for game in games
        .iter()
        .filter(|g| g.favorite || g.effective_rating().unwrap_or(0.0) >= 8.0)
    {
        for tag in game.tags.iter().chain(game.metadata.genres.iter()) {
            *tags.entry(tag.to_lowercase()).or_insert(0) += 1;
        }
    }
    tags
}

fn favorite_developers(games: &[Game]) -> HashSet<String> {
    games
        .iter()
        .filter(|g| g.favorite || g.effective_rating().unwrap_or(0.0) >= 8.0)
        .filter_map(|g| g.metadata.developer.as_ref())
        .map(|dev| dev.to_lowercase())
        .collect()
}

fn shared_tags(a: &Game, b: &Game) -> Vec<String> {
    let a_tags = a
        .tags
        .iter()
        .chain(a.metadata.genres.iter())
        .map(|tag| tag.to_lowercase())
        .collect::<HashSet<_>>();

    let mut shared = b
        .tags
        .iter()
        .chain(b.metadata.genres.iter())
        .filter(|tag| a_tags.contains(&tag.to_lowercase()))
        .cloned()
        .collect::<Vec<_>>();
    shared.sort();
    shared.dedup();
    shared
}
