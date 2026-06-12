// 萌游 MoeGame · 多源刮削结果合并引擎（M3）
//
// 功能：
//   1. 字段权重表：每个源对每个字段的信任权重（0.0-1.0）
//   2. 去重：按标题/别名相似度分组，每组选出最佳结果
//   3. 合并：同一游戏的多个源结果，逐字段择优合并
//   4. 评分：综合 confidence + 源权重 + 字段完整度排序

use crate::models::{ScrapeDetail, ScrapeResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// 字段权重表
// ============================================================================

/// 每个源对各字段的信任权重（0.0=不信任，1.0=完全信任）。
/// 基于经验：VNDB 最权威，Bangumi 中文友好，Steam 封面精准，DLsite 标签丰富。
#[derive(Debug, Clone)]
pub struct FieldWeights {
    pub title: f64,
    pub description: f64,
    pub cover: f64,
    pub background: f64,
    pub tags: f64,
    pub rating: f64,
    pub release_year: f64,
    pub developer: f64,
    pub publisher: f64,
    pub genres: f64,
    pub screenshots: f64,
    pub homepage: f64,
    pub age_rating: f64,
    pub aliases: f64,
}

impl Default for FieldWeights {
    fn default() -> Self {
        Self {
            title: 0.5,
            description: 0.5,
            cover: 0.5,
            background: 0.5,
            tags: 0.5,
            rating: 0.5,
            release_year: 0.5,
            developer: 0.5,
            publisher: 0.5,
            genres: 0.5,
            screenshots: 0.5,
            homepage: 0.5,
            age_rating: 0.5,
            aliases: 0.5,
        }
    }
}

/// 各源预设权重（基于经验调优）。
fn source_weights(source: &str) -> FieldWeights {
    match source {
        "vndb" => FieldWeights {
            title: 0.9,
            description: 0.8,
            cover: 0.9,
            background: 0.5,
            tags: 0.9,
            rating: 0.95,
            release_year: 0.95,
            developer: 0.95,
            publisher: 0.9,
            genres: 0.8,
            screenshots: 0.9,
            homepage: 0.7,
            age_rating: 0.7,
            aliases: 0.95,
        },
        "bangumi" => FieldWeights {
            title: 0.85,
            description: 0.75,
            cover: 0.7,
            background: 0.3,
            tags: 0.8,
            rating: 0.9,
            release_year: 0.85,
            developer: 0.7,
            publisher: 0.6,
            genres: 0.7,
            screenshots: 0.3,
            homepage: 0.6,
            age_rating: 0.4,
            aliases: 0.85,
        },
        "steam" => FieldWeights {
            title: 0.8,
            description: 0.7,
            cover: 0.95,      // Steam 封面最稳定
            background: 0.95, // Steam 背景图最稳
            tags: 0.6,
            rating: 0.7,
            release_year: 0.85,
            developer: 0.85,
            publisher: 0.85,
            genres: 0.6,
            screenshots: 0.9,
            homepage: 0.8,
            age_rating: 0.5,
            aliases: 0.5,
        },
        "dlsite" => FieldWeights {
            title: 0.8,
            description: 0.7,
            cover: 0.8,
            background: 0.4,
            tags: 0.95, // DLsite 标签最细
            rating: 0.5,
            release_year: 0.8,
            developer: 0.8,
            publisher: 0.7,
            genres: 0.85,
            screenshots: 0.7,
            homepage: 0.7,
            age_rating: 0.9, // DLsite 年龄分级准
            aliases: 0.5,
        },
        "kungal" | "touchgal" => FieldWeights {
            title: 0.8,
            description: 0.7,
            cover: 0.8,
            background: 0.4,
            tags: 0.85,
            rating: 0.75,
            release_year: 0.8,
            developer: 0.8,
            publisher: 0.6,
            genres: 0.75,
            screenshots: 0.6,
            homepage: 0.6,
            age_rating: 0.7,
            aliases: 0.7,
        },
        "ymgal" => FieldWeights {
            title: 0.8,
            description: 0.7,
            cover: 0.8,
            background: 0.4,
            tags: 0.8,
            rating: 0.8,
            release_year: 0.8,
            developer: 0.85,
            publisher: 0.5,
            genres: 0.7,
            screenshots: 0.4,
            homepage: 0.5,
            age_rating: 0.5,
            aliases: 0.7,
        },
        "erogamescape" => FieldWeights {
            title: 0.7,
            description: 0.6,
            cover: 0.3,
            background: 0.2,
            tags: 0.8,
            rating: 0.9, // 批评空间评分权威
            release_year: 0.7,
            developer: 0.7,
            publisher: 0.5,
            genres: 0.6,
            screenshots: 0.1,
            homepage: 0.4,
            age_rating: 0.5,
            aliases: 0.4,
        },
        "pcgw" => FieldWeights {
            title: 0.6,
            description: 0.6,
            cover: 0.5,
            background: 0.3,
            tags: 0.3,
            rating: 0.2,
            release_year: 0.5,
            developer: 0.6,
            publisher: 0.6,
            genres: 0.4,
            screenshots: 0.2,
            homepage: 0.8, // PCGW 链接准确
            age_rating: 0.2,
            aliases: 0.3,
        },
        "ai" => FieldWeights {
            title: 0.4,
            description: 0.5,
            cover: 0.2,
            background: 0.3,
            tags: 0.5,
            rating: 0.1,
            release_year: 0.3,
            developer: 0.3,
            publisher: 0.3,
            genres: 0.4,
            screenshots: 0.0,
            homepage: 0.2,
            age_rating: 0.3,
            aliases: 0.3,
        },
        _ => FieldWeights::default(),
    }
}

// ============================================================================
// 去重：判断两个刮削结果是否指向同一游戏
// ============================================================================

/// 标题相似度阈值。超过此值认为指向同一游戏。
const DEDUP_SIMILARITY_THRESHOLD: f64 = 0.75;

/// 判断两个结果是否指向同一游戏。
fn is_same_game(a: &ScrapeResult, b: &ScrapeResult) -> bool {
    // 同名来源 + 同 source_id → 严格相同
    if a.source == b.source && a.source_id == b.source_id && !a.source_id.is_empty() {
        return true;
    }
    // 标题相似度
    let sim = string_similarity(&a.title, &b.title);
    if sim >= DEDUP_SIMILARITY_THRESHOLD {
        return true;
    }
    // 检查别名交叉匹配
    if let Some(ref da) = a.detail {
        if let Some(ref db) = b.detail {
            for alias_a in &da.aliases {
                if string_similarity(alias_a, &b.title) >= DEDUP_SIMILARITY_THRESHOLD {
                    return true;
                }
                for alias_b in &db.aliases {
                    if string_similarity(alias_a, alias_b) >= DEDUP_SIMILARITY_THRESHOLD {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// 简单字符串相似度（字符级 Jaccard + 归一化）。
fn string_similarity(a: &str, b: &str) -> f64 {
    let a = a.to_lowercase();
    let b = b.to_lowercase();
    if a == b {
        return 1.0;
    }
    if a.contains(&b) || b.contains(&a) {
        return 0.9;
    }
    // 字符集 Jaccard
    let set_a: std::collections::HashSet<char> = a.chars().collect();
    let set_b: std::collections::HashSet<char> = b.chars().collect();
    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();
    if union == 0 {
        return 0.0;
    }
    intersection as f64 / union as f64
}

// ============================================================================
// 合并引擎
// ============================================================================

/// 合并配置。
#[derive(Debug, Clone)]
pub struct MergeConfig {
    /// 每个源至少需要多少条结果才参与合并
    pub min_results_per_source: usize,
    /// 是否保留原始多源结果（不合并）
    pub keep_raw: bool,
    /// 最大返回结果数
    pub max_results: usize,
}

impl Default for MergeConfig {
    fn default() -> Self {
        Self {
            min_results_per_source: 1,
            keep_raw: false,
            max_results: 20,
        }
    }
}

/// 合并后的结果，带来源追踪。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedResult {
    pub result: ScrapeResult,
    /// 各字段来自哪个源
    pub field_sources: HashMap<String, String>,
    /// 此组包含的原始结果数
    pub source_count: usize,
    /// 综合评分
    pub score: f64,
}

/// 主合并入口：
/// 1. 按游戏去重分组
/// 2. 每组合并各源的最优字段
/// 3. 按综合评分排序
pub fn merge_results(results: Vec<ScrapeResult>, config: &MergeConfig) -> Vec<MergedResult> {
    if results.is_empty() {
        return vec![];
    }

    // Step 1: 分组去重
    let groups = deduplicate(results);

    // Step 2: 每组合并
    let merged: Vec<MergedResult> = groups.into_iter().map(merge_group).collect();

    // Step 3: 按评分排序
    let mut sorted = merged;
    sorted.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Step 4: 截断
    sorted.truncate(config.max_results);

    sorted
}

/// 将结果按游戏分组。
fn deduplicate(results: Vec<ScrapeResult>) -> Vec<Vec<ScrapeResult>> {
    let mut groups: Vec<Vec<ScrapeResult>> = vec![];

    for result in results {
        let mut found_at: Option<usize> = None;
        for (i, group) in groups.iter().enumerate() {
            if group.iter().any(|g| is_same_game(g, &result)) {
                found_at = Some(i);
                break;
            }
        }
        match found_at {
            Some(i) => groups[i].push(result),
            None => groups.push(vec![result]),
        }
    }

    groups
}

/// 合并一组结果：选取每字段的最优值。
fn merge_group(group: Vec<ScrapeResult>) -> MergedResult {
    if group.len() == 1 {
        let r = group.into_iter().next().unwrap();
        let score = calc_result_score(&r, &source_weights(&r.source));
        return MergedResult {
            result: r.clone(),
            field_sources: all_from_source(&r.source),
            source_count: 1,
            score,
        };
    }

    let mut field_sources: HashMap<String, String> = HashMap::new();

    // 逐字段择优
    let title = pick_best_field_str(
        &group,
        |r| Some(r.title.clone()),
        |s| source_weights(s).title,
    );
    record_source(&mut field_sources, "title", &title.1);

    let description = pick_best_field(
        &group,
        |r| r.description.clone(),
        |s| source_weights(s).description,
    );
    record_source(&mut field_sources, "description", &description.1);

    let cover = pick_best_field(&group, |r| r.cover.clone(), |s| source_weights(s).cover);
    record_source(&mut field_sources, "cover", &cover.1);

    let background = pick_best_field(
        &group,
        |r| r.background.clone(),
        |s| source_weights(s).background,
    );
    record_source(&mut field_sources, "background", &background.1);

    let rating = pick_best_field(&group, |r| r.rating, |s| source_weights(s).rating);
    record_source(&mut field_sources, "rating", &rating.1);

    let release_year = pick_best_field(
        &group,
        |r| r.release_year,
        |s| source_weights(s).release_year,
    );
    record_source(&mut field_sources, "release_year", &release_year.1);

    // 标签：合并去重
    let mut all_tags: Vec<String> = vec![];
    let mut tag_sources: Vec<String> = vec![];
    for r in &group {
        let w = source_weights(&r.source).tags;
        if w >= 0.5 {
            for t in &r.tags {
                if !all_tags.contains(t) {
                    all_tags.push(t.clone());
                    tag_sources.push(r.source.clone());
                }
            }
        }
    }
    field_sources.insert("tags".to_string(), tag_sources.join(","));

    // Detail 字段择优
    let mut detail = ScrapeDetail::default();

    let developer = pick_detail_field(
        &group,
        |d| d.developer.clone(),
        |s| source_weights(s).developer,
    );
    detail.developer = developer.0;
    record_source(&mut field_sources, "developer", &developer.1);

    let publisher = pick_detail_field(
        &group,
        |d| d.publisher.clone(),
        |s| source_weights(s).publisher,
    );
    detail.publisher = publisher.0;
    record_source(&mut field_sources, "publisher", &publisher.1);

    let genres = pick_detail_field(
        &group,
        |d| {
            if d.genres.is_empty() {
                None
            } else {
                Some(d.genres.clone())
            }
        },
        |s| source_weights(s).genres,
    );
    detail.genres = genres.0.unwrap_or_default();
    record_source(&mut field_sources, "genres", &genres.1);

    let homepage = pick_detail_field(
        &group,
        |d| d.homepage.clone(),
        |s| source_weights(s).homepage,
    );
    detail.homepage = homepage.0;
    record_source(&mut field_sources, "homepage", &homepage.1);

    let age_rating = pick_detail_field(
        &group,
        |d| d.age_rating.clone(),
        |s| source_weights(s).age_rating,
    );
    detail.age_rating = age_rating.0;
    record_source(&mut field_sources, "age_rating", &age_rating.1);

    // 别名：合并去重
    let mut all_aliases: Vec<String> = vec![];
    for r in &group {
        if let Some(ref d) = r.detail {
            for a in &d.aliases {
                if !all_aliases.contains(a) {
                    all_aliases.push(a.clone());
                }
            }
        }
    }
    detail.aliases = all_aliases;

    // 截图：合并
    let mut all_screenshots: Vec<String> = vec![];
    for r in &group {
        let w = source_weights(&r.source).screenshots;
        if w >= 0.5 {
            if let Some(ref d) = r.detail {
                for s in &d.screenshots {
                    if !all_screenshots.contains(s) {
                        all_screenshots.push(s.clone());
                    }
                }
            }
        }
    }
    detail.screenshots = all_screenshots;

    // 语言：合并
    let mut all_langs: Vec<String> = vec![];
    let mut all_voice: Vec<String> = vec![];
    for r in &group {
        if let Some(ref d) = r.detail {
            for l in &d.languages {
                if !all_langs.contains(l) {
                    all_langs.push(l.clone());
                }
            }
            for v in &d.voice_languages {
                if !all_voice.contains(v) {
                    all_voice.push(v.clone());
                }
            }
        }
    }
    detail.languages = all_langs;
    detail.voice_languages = all_voice;

    // 选最佳 source_id
    let best_source = &group[0].source;
    let best_id = &group[0].source_id;

    let merged_result = ScrapeResult {
        title: title.0,
        description: description.0,
        cover: cover.0,
        background: background.0,
        tags: all_tags,
        rating: rating.0,
        release_year: release_year.0,
        source: format!("merged({})", best_source),
        source_id: best_id.clone(),
        detail: Some(detail),
    };

    // 综合评分
    let total_score = calc_merged_score(&group, &merged_result, &field_sources);

    MergedResult {
        result: merged_result,
        field_sources,
        source_count: group.len(),
        score: total_score,
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 从一组结果中选出某字段的最佳值（按源权重）。返回 (最佳值, 来源名)。
fn pick_best_field<T: Clone>(
    group: &[ScrapeResult],
    getter: fn(&ScrapeResult) -> Option<T>,
    weight_fn: fn(&str) -> f64,
) -> (Option<T>, String) {
    let mut best_val: Option<T> = None;
    let mut best_source = "none".to_string();
    let mut best_weight = 0.0_f64;

    for r in group {
        if let Some(val) = getter(r) {
            let w = weight_fn(&r.source);
            if w > best_weight {
                best_weight = w;
                best_val = Some(val);
                best_source = r.source.clone();
            }
        }
    }

    (best_val, best_source)
}

/// 字符串版（title 是 String 非 Option<String>）。
fn pick_best_field_str(
    group: &[ScrapeResult],
    getter: fn(&ScrapeResult) -> Option<String>,
    weight_fn: fn(&str) -> f64,
) -> (String, String) {
    let mut best_val = String::new();
    let mut best_source = "none".to_string();
    let mut best_weight = 0.0_f64;

    for r in group {
        if let Some(val) = getter(r) {
            let w = weight_fn(&r.source);
            if w > best_weight {
                best_weight = w;
                best_val = val;
                best_source = r.source.clone();
            }
        }
    }

    // 如果没找到任何值，取第一个结果的 title
    if best_val.is_empty() && !group.is_empty() {
        best_val = group[0].title.clone();
        best_source = group[0].source.clone();
    }

    (best_val, best_source)
}

/// 从 detail 字段择优。
fn pick_detail_field<T: Clone>(
    group: &[ScrapeResult],
    detail_getter: fn(&ScrapeDetail) -> Option<T>,
    weight_fn: fn(&str) -> f64,
) -> (Option<T>, String) {
    let mut best_val: Option<T> = None;
    let mut best_source = "none".to_string();
    let mut best_weight = 0.0_f64;

    for r in group {
        if let Some(ref d) = r.detail {
            if let Some(val) = detail_getter(d) {
                let w = weight_fn(&r.source);
                if w > best_weight {
                    best_weight = w;
                    best_val = Some(val);
                    best_source = r.source.clone();
                }
            }
        }
    }

    (best_val, best_source)
}

fn record_source(map: &mut HashMap<String, String>, field: &str, source: &str) {
    map.insert(field.to_string(), source.to_string());
}

fn all_from_source(source: &str) -> HashMap<String, String> {
    let mut m = HashMap::new();
    for f in &[
        "title",
        "description",
        "cover",
        "background",
        "tags",
        "rating",
        "release_year",
        "developer",
        "publisher",
        "genres",
        "screenshots",
        "homepage",
        "age_rating",
        "aliases",
    ] {
        m.insert(f.to_string(), source.to_string());
    }
    m
}

fn calc_result_score(r: &ScrapeResult, w: &FieldWeights) -> f64 {
    let mut score = 0.0_f64;
    let mut count = 0.0_f64;
    if r.description.is_some() {
        score += w.description;
        count += 1.0;
    }
    if r.cover.is_some() {
        score += w.cover;
        count += 1.0;
    }
    if !r.tags.is_empty() {
        score += w.tags;
        count += 1.0;
    }
    if r.rating.is_some() {
        score += w.rating;
        count += 1.0;
    }
    if r.release_year.is_some() {
        score += w.release_year;
        count += 1.0;
    }
    if let Some(ref d) = r.detail {
        if d.developer.is_some() {
            score += w.developer;
            count += 1.0;
        }
        if !d.genres.is_empty() {
            score += w.genres;
            count += 1.0;
        }
        if !d.screenshots.is_empty() {
            score += w.screenshots;
            count += 1.0;
        }
    }
    if count > 0.0 {
        score / count
    } else {
        0.0
    }
}

fn calc_merged_score(
    group: &[ScrapeResult],
    merged: &ScrapeResult,
    field_sources: &HashMap<String, String>,
) -> f64 {
    let base = calc_result_score(merged, &FieldWeights::default());
    let diversity_bonus = (group.len() as f64).min(5.0) * 0.05; // 多源加成
    let source_bonus = if field_sources.values().any(|s| s == "vndb") {
        0.1
    } else {
        0.0
    };
    base + diversity_bonus + source_bonus
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(
        title: &str,
        source: &str,
        source_id: &str,
        desc: Option<&str>,
        rating: Option<f64>,
    ) -> ScrapeResult {
        ScrapeResult {
            title: title.to_string(),
            description: desc.map(|s| s.to_string()),
            cover: None,
            background: None,
            tags: vec![],
            rating,
            release_year: None,
            source: source.to_string(),
            source_id: source_id.to_string(),
            detail: None,
        }
    }

    #[test]
    fn test_string_similarity_exact() {
        assert_eq!(string_similarity("Steins;Gate", "Steins;Gate"), 1.0);
    }

    #[test]
    fn test_string_similarity_contains() {
        assert!(string_similarity("Steins;Gate", "Steins Gate") > 0.5);
    }

    #[test]
    fn test_dedup_same_game() {
        let a = make_result("CLANNAD", "vndb", "v1", Some("desc1"), Some(9.0));
        let b = make_result("CLANNAD", "bangumi", "bg1", Some("desc2"), Some(8.5));
        assert!(is_same_game(&a, &b));
    }

    #[test]
    fn test_dedup_different_game() {
        let a = make_result("CLANNAD", "vndb", "v1", None, None);
        let b = make_result("Steins;Gate", "vndb", "v2", None, None);
        assert!(!is_same_game(&a, &b));
    }

    #[test]
    fn test_merge_picks_best_rating() {
        let a = make_result("Test", "vndb", "v1", Some("VNDB desc"), Some(9.0));
        let b = make_result("Test", "bangumi", "bg1", Some("Bangumi desc"), Some(8.0));
        let merged = merge_results(vec![a, b], &MergeConfig::default());
        assert_eq!(merged.len(), 1);
        // VNDB 评分权重 > Bangumi → 应选 9.0
        assert_eq!(merged[0].result.rating, Some(9.0));
        // VNDB 描述权重 > Bangumi → 应选 VNDB desc
        assert_eq!(merged[0].result.description, Some("VNDB desc".to_string()));
    }

    #[test]
    fn test_merge_different_games_kept_separate() {
        let a = make_result("CLANNAD", "vndb", "v1", None, None);
        let b = make_result("Steins;Gate", "vndb", "v2", None, None);
        let merged = merge_results(vec![a, b], &MergeConfig::default());
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn test_tag_merge_dedup() {
        let mut a = make_result("Test", "vndb", "v1", None, None);
        a.tags = vec!["Visual Novel".into(), "Drama".into()];
        let mut b = make_result("Test", "dlsite", "d1", None, None);
        b.tags = vec!["Visual Novel".into(), "Romance".into()];
        let merged = merge_results(vec![a, b], &MergeConfig::default());
        assert_eq!(merged.len(), 1);
        let tags = &merged[0].result.tags;
        assert!(tags.contains(&"Visual Novel".to_string()));
        assert!(tags.contains(&"Drama".to_string()));
        assert!(tags.contains(&"Romance".to_string()));
        assert_eq!(tags.len(), 3); // "Visual Novel" deduped
    }
}
