//! 游戏元数据刮削模块

pub mod ai;
pub mod ai_presets;
pub mod bangumi;
pub mod cache;
pub mod dlsite;
pub mod erogamescape;
pub mod error;
pub mod kungal;
pub mod merge;
pub mod pcgw;
pub mod screenshots;
pub mod steam;
pub mod strategy;
mod touchgal;
pub mod utils;
pub mod vndb;
pub mod ymgal;

pub use ai::AiScrapeConfig;
pub use vndb::VndbDetail;

use crate::models::{ScrapeResult, ScrapeSourceStatus};
use cache::ScrapeCache;

/// 单个数据源刮削任务的 JoinHandle（避免手写超长元组类型触发 clippy::type_complexity）
type ScrapeJoinHandle = tokio::task::JoinHandle<Result<Vec<ScrapeResult>, String>>;

/// 全局刮削缓存（1h TTL）
static CACHE: std::sync::LazyLock<ScrapeCache> =
    std::sync::LazyLock::new(|| ScrapeCache::new(3600));

pub fn global_cache() -> &'static ScrapeCache {
    &CACHE
}

/// 并发搜索所有已启用的数据源（带缓存 + 超时优化）
/// 返回 (搜索结果, 各源状态) 以便前端展示哪些源成功/失败
pub async fn search_all(
    query: &str,
    vndb_enabled: bool,
    bangumi_enabled: bool,
    dlsite_enabled: bool,
    touchgal_enabled: bool,
    erogamescape_enabled: bool,
    ymgal_enabled: bool,
    kungal_enabled: bool,
    steam_enabled: bool,
    pcgw_enabled: bool,
) -> (Vec<ScrapeResult>, Vec<ScrapeSourceStatus>) {
    let cache = global_cache();
    let mut handles: Vec<(String, ScrapeJoinHandle)> = vec![];
    let mut cached_results: Vec<ScrapeResult> = vec![];
    let mut statuses: Vec<ScrapeSourceStatus> = vec![];

    macro_rules! spawn_source {
        ($enabled:expr, $name:expr, $search_fn:expr) => {
            if $enabled {
                let q = query.to_string();
                if let Some(cached) = cache.get(&q, $name) {
                    let count = cached.len();
                    cached_results.extend(cached);
                    statuses.push(ScrapeSourceStatus {
                        source: $name.to_string(),
                        ok: true,
                        count,
                        error: None,
                    });
                } else {
                    let q2 = q.clone();
                    let src_name = $name.to_string();
                    handles.push((
                        src_name,
                        tokio::spawn(async move {
                            let r = $search_fn(&q2).await;
                            if let Ok(ref results) = r {
                                cache.set(&q2, $name, results.clone());
                            }
                            r
                        }),
                    ));
                }
            }
        };
    }

    spawn_source!(vndb_enabled, "vndb", vndb::search_simple);
    spawn_source!(bangumi_enabled, "bangumi", bangumi::search_simple);
    spawn_source!(dlsite_enabled, "dlsite", dlsite::search_simple);
    spawn_source!(touchgal_enabled, "touchgal", touchgal::search_simple);
    spawn_source!(
        erogamescape_enabled,
        "erogamescape",
        erogamescape::search_simple
    );
    spawn_source!(ymgal_enabled, "ymgal", ymgal::search_simple);
    spawn_source!(kungal_enabled, "kungal", kungal::search_simple);
    spawn_source!(steam_enabled, "steam", steam::search_simple);
    spawn_source!(pcgw_enabled, "pcgw", pcgw::search_simple);

    for (name, h) in handles {
        match h.await {
            Ok(Ok(mut r)) => {
                let count = r.len();
                cached_results.append(&mut r);
                statuses.push(ScrapeSourceStatus {
                    source: name,
                    ok: true,
                    count,
                    error: None,
                });
            }
            Ok(Err(e)) => {
                tracing::warn!(source = %name, error = %e, "Scrape source failed");
                statuses.push(ScrapeSourceStatus {
                    source: name,
                    ok: false,
                    count: 0,
                    error: Some(e),
                });
            }
            Err(e) => {
                tracing::warn!(source = %name, error = %e, "Scrape task panicked");
                statuses.push(ScrapeSourceStatus {
                    source: name,
                    ok: false,
                    count: 0,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    (cached_results, statuses)
}

/// AI 增强刮削
pub async fn scrape_game(
    query: &str,
    vndb_enabled: bool,
    bangumi_enabled: bool,
    dlsite_enabled: bool,
    touchgal_enabled: bool,
    erogamescape_enabled: bool,
    ymgal_enabled: bool,
    kungal_enabled: bool,
    steam_enabled: bool,
    pcgw_enabled: bool,
    ai_config: Option<&AiScrapeConfig>,
) -> (Vec<ScrapeResult>, Vec<ScrapeSourceStatus>) {
    let (mut results, statuses) = search_all(
        query,
        vndb_enabled,
        bangumi_enabled,
        dlsite_enabled,
        touchgal_enabled,
        erogamescape_enabled,
        ymgal_enabled,
        kungal_enabled,
        steam_enabled,
        pcgw_enabled,
    )
    .await;

    if results.is_empty() {
        if let Some(config) = ai_config {
            if let Ok(ai_meta) = ai::enhance(config, query, None, &[]).await {
                // 尝试从本地 Steam librarycache 匹配封面（远程源全部被墙时的兜底）
                let cover = crate::integration::find_local_steam_cover_by_name(query);
                results.push(ScrapeResult {
                    title: query.to_string(),
                    description: ai_meta.description,
                    cover,
                    background: ai_meta.background,
                    tags: ai_meta.tags,
                    rating: None,
                    release_year: None,
                    source: "ai".to_string(),
                    source_id: format!("ai:{}", query),
                    detail: None,
                });
            }
        }
        // 所有远程源失败且无 AI：尝试仅从本地 Steam librarycache 返回封面
        if results.is_empty() {
            if let Some(local_cover) = crate::integration::find_local_steam_cover_by_name(query) {
                results.push(ScrapeResult {
                    title: query.to_string(),
                    description: None,
                    cover: Some(local_cover),
                    background: None,
                    tags: vec![],
                    rating: None,
                    release_year: None,
                    source: "local".to_string(),
                    source_id: query.to_string(),
                    detail: None,
                });
            }
        }
        return (results, statuses);
    }

    if let Some(config) = ai_config {
        let enhance_count = results.len().min(5);
        let mut enhanced = vec![];
        for result in results.iter().take(enhance_count) {
            match ai::enhance(
                config,
                &result.title,
                result.description.as_deref(),
                &result.tags,
            )
            .await
            {
                Ok(ai_meta) => {
                    let mut r = result.clone();
                    if ai_meta.description.is_some() {
                        r.description = ai_meta.description;
                    }
                    let mut all_tags: Vec<String> = result.tags.clone();
                    for tag in &ai_meta.tags {
                        if !all_tags
                            .iter()
                            .any(|t| t.to_lowercase() == tag.to_lowercase())
                        {
                            all_tags.push(tag.clone());
                        }
                    }
                    r.tags = all_tags;
                    r.background = ai_meta.background;
                    r.source = format!("{}+ai", result.source);
                    enhanced.push(r);
                }
                Err(e) => {
                    tracing::warn!(error = %e, "AI enhance failed");
                    enhanced.push(result.clone());
                }
            }
        }
        enhanced.extend(results[enhance_count..].iter().cloned());
        results = enhanced;
    }

    // 补缺封面：远程源有结果但无封面的，尝试本地 Steam librarycache
    for r in &mut results {
        if r.cover.is_none() {
            r.cover = crate::integration::find_local_steam_cover_by_name(&r.title);
        }
    }

    (results, statuses)
}
