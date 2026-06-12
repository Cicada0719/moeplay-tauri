//! NSFW 内容过滤
//!
//! 根据游戏名称、标签、结构化元数据和刮削详情判断内容分级，并给出 UI 显示决策。

use serde::{Deserialize, Serialize};

use crate::models::Game;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NsfwDisplayMode {
    Show,
    Blur,
    Hide,
}

impl NsfwDisplayMode {
    pub fn parse(value: Option<&str>) -> Self {
        match value.unwrap_or("blur").to_lowercase().as_str() {
            "show" => Self::Show,
            "hide" => Self::Hide,
            _ => Self::Blur,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Show => "show",
            Self::Blur => "blur",
            Self::Hide => "hide",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NsfwDecision {
    pub is_nsfw: bool,
    pub display_mode: NsfwDisplayMode,
    pub should_show: bool,
    pub should_blur: bool,
    pub reasons: Vec<String>,
}

const NSFW_TAG_KEYWORDS: &[&str] = &[
    "nsfw",
    "adult",
    "hentai",
    "eroge",
    "r18",
    "r-18",
    "sexual",
    "nudity",
    "18+",
    "mature",
    "explicit",
    "x-rated",
    "成人向け",
    "エロ",
    "抜きゲー",
    "アダルト",
    "成人",
    "18禁",
    "工口",
    "拔作",
    "限制级",
    "黄油",
    "色气",
];

const NSFW_RATINGS: &[&str] = &[
    "nsfw",
    "r18",
    "r-18",
    "18+",
    "adult",
    "mature",
    "restricted",
    "成人",
    "18禁",
];

const NAME_MARKERS: &[&str] = &[
    "[r18]",
    "[r-18]",
    "[nsfw]",
    "[18+]",
    "[成人]",
    "【r18】",
    "【成人】",
];

pub fn decide(game: &Game, mode: NsfwDisplayMode) -> NsfwDecision {
    let reasons = reasons(game);
    let is_nsfw = !reasons.is_empty();
    let should_show = !is_nsfw || mode != NsfwDisplayMode::Hide;
    let should_blur = is_nsfw && mode == NsfwDisplayMode::Blur;

    NsfwDecision {
        is_nsfw,
        display_mode: mode,
        should_show,
        should_blur,
        reasons,
    }
}

pub fn is_nsfw(game: &Game) -> bool {
    !reasons(game).is_empty()
}

pub fn filter_games(games: Vec<Game>, mode: NsfwDisplayMode) -> Vec<Game> {
    if mode != NsfwDisplayMode::Hide {
        return games;
    }

    games.into_iter().filter(|game| !is_nsfw(game)).collect()
}

fn reasons(game: &Game) -> Vec<String> {
    let mut reasons = Vec::new();

    let lower_name = game.name.to_lowercase();
    if NAME_MARKERS
        .iter()
        .any(|marker| lower_name.contains(marker))
    {
        reasons.push("name_marker".to_string());
    }

    for tag in &game.tags {
        if is_nsfw_keyword(tag) {
            reasons.push(format!("tag:{}", tag));
        }
    }

    for tag in &game.tag_entries {
        if is_nsfw_keyword(&tag.name) {
            reasons.push(format!("tag_entry:{}", tag.name));
        }
    }

    for genre in &game.metadata.genres {
        if is_nsfw_keyword(genre) {
            reasons.push(format!("genre:{}", genre));
        }
    }

    if let Some(ref age_rating) = game.metadata.age_rating {
        if is_nsfw_rating(age_rating) {
            reasons.push(format!("age_rating:{}", age_rating));
        }
    }

    if let Some(ref original_name) = game.metadata.original_name {
        let lower = original_name.to_lowercase();
        if NAME_MARKERS.iter().any(|marker| lower.contains(marker)) {
            reasons.push("original_name_marker".to_string());
        }
    }

    reasons.sort();
    reasons.dedup();
    reasons
}

fn is_nsfw_keyword(value: &str) -> bool {
    let normalized = normalize(value);
    NSFW_TAG_KEYWORDS
        .iter()
        .any(|keyword| normalized == normalize(keyword) || normalized.contains(&normalize(keyword)))
}

fn is_nsfw_rating(value: &str) -> bool {
    let normalized = normalize(value);
    NSFW_RATINGS
        .iter()
        .any(|rating| normalized == normalize(rating) || normalized.contains(&normalize(rating)))
}

fn normalize(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '_' && *c != '-')
        .collect()
}
