use crate::db::Database;
use crate::models::{Game, ScrapeResponse, ScrapeResult};
use crate::scraper;
use tauri::State;

// ===== 刮削命令 =====

#[tauri::command]
pub async fn scrape_games(
    query: String,
    vndb: bool,
    bangumi: bool,
    dlsite: Option<bool>,
    touchgal: Option<bool>,
    erogamescape: Option<bool>,
    ymgal: Option<bool>,
    kungal: Option<bool>,
    steam: Option<bool>,
    pcgw: Option<bool>,
    db: State<'_, Database>,
) -> Result<ScrapeResponse, String> {
    let dlsite = dlsite.unwrap_or(false);
    let touchgal = touchgal.unwrap_or(false);
    let erogamescape = erogamescape.unwrap_or(false);
    let ymgal = ymgal.unwrap_or(false);
    let kungal = kungal.unwrap_or(false);
    let steam = steam.unwrap_or(false);
    let pcgw = pcgw.unwrap_or(false);

    if !vndb
        && !bangumi
        && !dlsite
        && !touchgal
        && !erogamescape
        && !ymgal
        && !kungal
        && !steam
        && !pcgw
    {
        return Err("请至少启用一个数据源".to_string());
    }

    let settings = db.get_settings();
    let proxy = if settings.scraper_proxy.trim().is_empty() { None } else { Some(settings.scraper_proxy.clone()) };
    scraper::utils::set_proxy(proxy);

    let (results, source_status) = scraper::search_all(
        &query,
        vndb,
        bangumi,
        dlsite,
        touchgal,
        erogamescape,
        ymgal,
        kungal,
        steam,
        pcgw,
    )
    .await;
    Ok(ScrapeResponse { results, source_status })
}

/// 按 DLsite 产品 ID 获取详情
#[tauri::command]
pub async fn scrape_dlsite_product(product_id: String) -> Result<ScrapeResult, String> {
    scraper::dlsite::get_product(&product_id)
        .await
        .map_err(|e| e.to_string())
}

/// 按 ErogameScape 游戏 ID 获取详情
#[tauri::command]
pub async fn scrape_erogamescape_game(game_id: String) -> Result<ScrapeResult, String> {
    scraper::erogamescape::get_game(&game_id)
        .await
        .map_err(|e| e.to_string())
}

/// 按 Ymgal 游戏 ID 获取详情
#[tauri::command]
pub async fn scrape_ymgal_detail(game_id: String) -> Result<ScrapeResult, String> {
    scraper::ymgal::get_detail(&game_id)
        .await
        .map_err(|e| e.to_string())
}

/// 按 Kungal 游戏 ID 获取详情
#[tauri::command]
pub async fn scrape_kungal_detail(game_id: String) -> Result<ScrapeResult, String> {
    scraper::kungal::get_detail(&game_id)
        .await
        .map_err(|e| e.to_string())
}

/// 按 Steam App ID 获取详情
#[tauri::command]
pub async fn scrape_steam_app(app_id: String) -> Result<ScrapeResult, String> {
    scraper::steam::get_app_details(&app_id)
        .await
        .map_err(|e| e.to_string())
}

/// 按 PCGamingWiki 标题获取技术资料
#[tauri::command]
pub async fn scrape_pcgw_page(title: String) -> Result<ScrapeResult, String> {
    scraper::pcgw::get_summary(&title)
        .await
        .map_err(|e| e.to_string())
}

/// AI 增强刮削：多源并行搜索 + LLM 补全
#[tauri::command]
pub async fn scrape_game(
    query: String,
    vndb: bool,
    bangumi: bool,
    dlsite: Option<bool>,
    touchgal: Option<bool>,
    erogamescape: Option<bool>,
    ymgal: Option<bool>,
    kungal: Option<bool>,
    steam: Option<bool>,
    pcgw: Option<bool>,
    db: State<'_, Database>,
) -> Result<ScrapeResponse, String> {
    let dlsite = dlsite.unwrap_or(false);
    let touchgal = touchgal.unwrap_or(false);
    let erogamescape = erogamescape.unwrap_or(false);
    let ymgal = ymgal.unwrap_or(false);
    let kungal = kungal.unwrap_or(false);
    let steam = steam.unwrap_or(false);
    let pcgw = pcgw.unwrap_or(false);

    if !vndb
        && !bangumi
        && !dlsite
        && !touchgal
        && !erogamescape
        && !ymgal
        && !kungal
        && !steam
        && !pcgw
    {
        return Err("请至少启用一个数据源".to_string());
    }

    let settings = db.get_settings();
    let proxy = if settings.scraper_proxy.trim().is_empty() { None } else { Some(settings.scraper_proxy.clone()) };
    scraper::utils::set_proxy(proxy);

    let ai_config = if settings.ai_enabled && !settings.ai_api_key.is_empty() {
        Some(scraper::AiScrapeConfig {
            api_url: settings.ai_api_url.clone(),
            api_key: settings.ai_api_key.clone(),
            model: settings.ai_model.clone(),
        })
    } else {
        None
    };

    let (mut results, source_status) = scraper::scrape_game(
        &query,
        vndb,
        bangumi,
        dlsite,
        touchgal,
        erogamescape,
        ymgal,
        kungal,
        steam,
        pcgw,
        ai_config.as_ref(),
    )
    .await;

    for r in &mut results {
        if let Some(ref url) = r.cover {
            if url.starts_with("http") {
                r.cover = Some(crate::commands::fetch_cover_to_local(url, &r.source_id).await);
            }
        }
        if let Some(ref url) = r.background {
            if url.starts_with("http") {
                r.background = Some(crate::commands::fetch_cover_to_local(url, &format!("{}_bg", r.source_id)).await);
            }
        }
    }

    Ok(ScrapeResponse { results, source_status })
}

/// 应用刮削结果到游戏记录
#[tauri::command]
pub fn apply_scrape_result(
    db: State<'_, Database>,
    game_id: String,
    result: ScrapeResult,
) -> Result<Game, String> {
    let ScrapeResult {
        title,
        description,
        cover,
        background,
        tags,
        rating,
        release_year,
        source,
        source_id,
        detail,
    } = result;
    let detail = detail.unwrap_or_default();
    let aliases = (!detail.aliases.is_empty()).then_some(detail.aliases);
    let genres = (!detail.genres.is_empty()).then_some(detail.genres);
    let languages = (!detail.languages.is_empty()).then_some(detail.languages);
    let voice_languages = (!detail.voice_languages.is_empty()).then_some(detail.voice_languages);
    let screenshots = (!detail.screenshots.is_empty()).then_some(detail.screenshots);

    db.apply_scrape_result_ext(
        &game_id,
        Some(title),
        description,
        cover,
        background,
        Some(tags),
        rating,
        release_year,
        Some(source.as_str()),
        Some(source_id),
        detail.developer,
        detail.publisher,
        genres,
        languages,
        detail.engine,
        detail.age_rating,
        detail.series,
        detail.release_date,
        voice_languages,
        aliases,
        screenshots,
        detail.homepage,
    )
}

#[tauri::command]
pub async fn fetch_vndb_detail(id: String) -> Result<scraper::VndbDetail, String> {
    scraper::vndb::detail(&id).await.map_err(|e| e.to_string())
}

/// Bangumi 详情获取
#[tauri::command]
pub async fn fetch_bangumi_detail(id: String) -> Result<ScrapeResult, String> {
    scraper::bangumi::detail(&id).await
}

fn vndb_detail_to_scrape_result(d: scraper::VndbDetail) -> ScrapeResult {
    let tags: Vec<String> = d
        .tags
        .iter()
        .filter(|t| t.category == "cont")
        .map(|t| t.name.clone())
        .take(20)
        .collect();
    let background = d.screenshots.first().cloned();
    let homepage = d
        .links
        .iter()
        .find(|l| {
            let lab = l.label.to_lowercase();
            lab.contains("official") || l.label.contains("官方") || lab.contains("home")
        })
        .map(|l| l.url.clone());
    let mut detail = crate::models::ScrapeDetail::default();
    detail.developer = d.developers.first().cloned();
    detail.aliases = d.aliases;
    detail.languages = d.languages;
    detail.screenshots = d.screenshots;
    detail.release_date = d.released;
    detail.homepage = homepage;
    ScrapeResult {
        title: d.title,
        description: d.description,
        cover: d.cover_url,
        background,
        tags,
        rating: d.rating,
        release_year: d.release_year,
        source: "vndb".to_string(),
        source_id: d.id,
        detail: Some(detail),
    }
}

/// 取某条搜索结果的「全量详情」（截图/开发商/发行商/流派/别名/发行日期/官网等），
/// 按 source 分派到各源已有的详情接口。供 ScrapeDialog 点选后补全用——
/// 搜索只回浅层结果，真正的富字段在各源独立详情接口里。
#[tauri::command]
pub async fn fetch_full_detail(source: String, source_id: String) -> Result<ScrapeResult, String> {
    let lower = source.to_lowercase();
    let s = lower.strip_suffix("+ai").unwrap_or(lower.as_str()).trim();
    tracing::info!(source = %s, %source_id, "fetch_full_detail: START");
    let result: Result<ScrapeResult, String> = match s {
        "vndb" => scraper::vndb::detail(&source_id)
            .await
            .map(vndb_detail_to_scrape_result)
            .map_err(|e| e.to_string()),
        "bangumi" => scraper::bangumi::detail(&source_id).await,
        "kungal" | "touchgal" => scraper::kungal::get_detail(&source_id)
            .await
            .map_err(|e| e.to_string()),
        "ymgal" => scraper::ymgal::get_detail(&source_id)
            .await
            .map_err(|e| e.to_string()),
        "dlsite" => scraper::dlsite::get_product(&source_id)
            .await
            .map_err(|e| e.to_string()),
        "erogamescape" | "egs" => scraper::erogamescape::get_game(&source_id)
            .await
            .map_err(|e| e.to_string()),
        "steam" => scraper::steam::get_app_details(&source_id)
            .await
            .map_err(|e| e.to_string()),
        "pcgw" => scraper::pcgw::get_summary(&source_id)
            .await
            .map_err(|e| e.to_string()),
        other => Err(format!("不支持的详情源: {}", other)),
    };
    match &result {
        Ok(r) => {
            let n_ss = r.detail.as_ref().map(|d| d.screenshots.len()).unwrap_or(0);
            let has_dev = r
                .detail
                .as_ref()
                .and_then(|d| d.developer.as_ref())
                .is_some();
            tracing::info!(source = %s, screenshots = n_ss, has_dev, "fetch_full_detail: OK");
        }
        Err(e) => tracing::warn!(source = %s, error = %e, "fetch_full_detail: FAILED"),
    }
    result
}

// ===== M3 合并刮削 =====

/// M3 合并刮削：多源搜索 → 合并 → AI 增强 → 返回
#[tauri::command]
pub async fn scrape_game_merged(
    db: State<'_, Database>,
    query: String,
    source_hint: Option<String>,
    strategy: Option<String>,
) -> Result<Vec<crate::scraper::merge::MergedResult>, String> {
    let settings = db.get_settings();
    let strat = match strategy.as_deref() {
        Some("incremental") => scraper::strategy::ScrapeStrategy::Incremental,
        Some("patch_missing") => scraper::strategy::ScrapeStrategy::PatchMissing,
        Some("retry_failed") => scraper::strategy::ScrapeStrategy::RetryFailed,
        _ => scraper::strategy::ScrapeStrategy::Full,
    };

    let proxy = if settings.scraper_proxy.trim().is_empty() { None } else { Some(settings.scraper_proxy.clone()) };
    scraper::utils::set_proxy(proxy);

    let route = scraper::strategy::ScrapeRouter::plan(source_hint.as_deref(), false, false);

    let (raw, _statuses) = scraper::search_all(
        &query,
        settings.vndb_enabled,
        settings.bangumi_enabled,
        settings.dlsite_enabled,
        settings.touchgal_enabled,
        settings.erogamescape_enabled,
        settings.ymgal_enabled,
        settings.kungal_enabled,
        settings.steam_enabled,
        settings.pcgw_enabled,
    )
    .await;

    let merge_config = scraper::merge::MergeConfig {
        max_results: 10,
        ..Default::default()
    };
    let merged = scraper::merge::merge_results(raw, &merge_config);

    let mut result = merged;
    if settings.ai_enabled && !settings.ai_api_key.is_empty() && route.with_ai {
        let config = scraper::AiScrapeConfig {
            api_url: settings.ai_api_url.clone(),
            api_key: settings.ai_api_key.clone(),
            model: settings.ai_model.clone(),
        };
        for mr in &mut result {
            if let Ok(enhanced) = crate::scraper::ai::enhance(
                &config,
                &mr.result.title,
                mr.result.description.as_deref(),
                &mr.result.tags,
            )
            .await
            {
                if enhanced.description.is_some() {
                    mr.result.description = enhanced.description;
                }
                for tag in enhanced.tags {
                    if !mr.result.tags.contains(&tag) {
                        mr.result.tags.push(tag);
                    }
                }
            }
        }
    }

    for mr in &mut result {
        if let Some(ref url) = mr.result.cover {
            if url.starts_with("http") {
                let local = crate::commands::fetch_cover_to_local(url, &mr.result.source_id).await;
                mr.result.cover = Some(local);
            }
        }
        if let Some(ref url) = mr.result.background {
            if url.starts_with("http") {
                let local = crate::commands::fetch_cover_to_local(url, &format!("{}_bg", mr.result.source_id)).await;
                mr.result.background = Some(local);
            }
        }
    }

    tracing::info!(query, results = result.len(), strategy = %strat, "M3 scrape completed");
    Ok(result)
}

/// 获取 AI Provider 列表
#[tauri::command]
pub fn get_ai_providers(
    db: State<'_, Database>,
) -> Result<Vec<scraper::ai_presets::AiProvider>, String> {
    let settings = db.get_settings();
    let mut providers = scraper::ai_presets::builtin_providers();
    if !settings.ai_api_key.is_empty() {
        for p in &mut providers {
            if p.id == "openai" {
                p.api_key = settings.ai_api_key.clone();
                p.api_url.clone_from(&settings.ai_api_url);
                p.default_model.clone_from(&settings.ai_model);
                p.enabled = settings.ai_enabled;
            }
        }
    }
    Ok(providers)
}

/// 获取 AI Preset 列表
#[tauri::command]
pub fn get_ai_presets() -> Vec<scraper::ai_presets::AiPreset> {
    scraper::ai_presets::builtin_presets()
}

/// 运行指定 preset 的 AI 推理
#[tauri::command]
pub async fn run_ai_preset(
    db: State<'_, Database>,
    preset_id: String,
    input: String,
) -> Result<String, String> {
    let settings = db.get_settings();
    let presets = scraper::ai_presets::builtin_presets();
    let preset = presets
        .iter()
        .find(|p| p.id == preset_id)
        .ok_or_else(|| "AI preset not found".to_string())?;

    if !settings.ai_enabled || settings.ai_api_key.trim().is_empty() {
        return Err("AI API key is not configured".to_string());
    }

    let mut provider = scraper::ai_presets::builtin_providers()
        .into_iter()
        .find(|p| p.id == preset.provider_id)
        .ok_or_else(|| format!("AI provider not found: {}", preset.provider_id))?;
    provider.api_url.clone_from(&settings.ai_api_url);
    provider.api_key.clone_from(&settings.ai_api_key);
    provider.default_model.clone_from(&settings.ai_model);
    provider.enabled = true;

    let model = preset
        .model_override
        .as_deref()
        .filter(|model| !model.trim().is_empty())
        .unwrap_or(&provider.default_model);

    scraper::ai_presets::call_llm(
        &provider,
        model,
        &preset.system_prompt,
        &input,
        preset.temperature,
        preset.max_tokens,
    )
    .await
}

#[tauri::command]
pub async fn download_screenshots(
    game_id: String,
    urls: Vec<String>,
    kind: Option<String>,
) -> Result<Vec<scraper::screenshots::DownloadedImage>, String> {
    let image_kind = match kind.unwrap_or_else(|| "screenshot".to_string()).as_str() {
        "cg" => scraper::screenshots::ImageKind::Cg,
        "cover" => scraper::screenshots::ImageKind::Cover,
        "background" => scraper::screenshots::ImageKind::Background,
        "character_standing" => scraper::screenshots::ImageKind::CharacterStanding,
        _ => scraper::screenshots::ImageKind::Screenshot,
    };

    Ok(scraper::screenshots::download_images(&urls, &game_id, image_kind).await)
}
