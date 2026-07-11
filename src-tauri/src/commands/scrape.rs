use crate::db::Database;
use crate::domain::{ProviderError, ProviderErrorKind};
use crate::models::{Game, ScrapeResponse, ScrapeResult};
use crate::scraper;
use crate::secret_store::{SecretKind, SecretStore};
use crate::task_queue::{JobOperation, TaskEventLevel, TaskQueue};
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
    queue: State<'_, TaskQueue>,
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

    let job = queue.enqueue_operation(
        "多源元数据搜索".to_string(),
        JobOperation::Scrape {
            game_id: "search".to_string(),
            provider_id: None,
        },
        None,
    )?;
    let cancellation = queue.register_operation(&job.id)?;
    queue.mark_running(
        &job.id,
        Some("正在并行查询已启用的数据源".to_string()),
        Some(0.05),
    )?;

    let settings = db.get_settings();
    let proxy = if settings.scraper_proxy.trim().is_empty() {
        None
    } else {
        Some(settings.scraper_proxy.clone())
    };
    scraper::utils::set_proxy(proxy);

    if cancellation.is_cancelled() {
        return Err("任务已取消".to_string());
    }
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
    if cancellation.is_cancelled() {
        return Err("任务已取消".to_string());
    }
    queue.append_event(
        &job.id,
        TaskEventLevel::Info,
        "scrape_sources_completed".to_string(),
        format!("完成 {} 个来源的查询", source_status.len()),
        Some(0.9),
    )?;
    queue.mark_succeeded(&job.id, Some(format!("找到 {} 条候选结果", results.len())))?;
    Ok(ScrapeResponse {
        results,
        source_status,
    })
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
    secret_store: State<'_, SecretStore>,
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

    let settings =
        super::settings::load_settings_with_secret_migration(db.inner(), secret_store.inner())?;
    let proxy = if settings.scraper_proxy.trim().is_empty() {
        None
    } else {
        Some(settings.scraper_proxy.clone())
    };
    scraper::utils::set_proxy(proxy);

    let ai_config = if settings.ai_enabled {
        let ai_api_key = ai_api_key_for_settings(secret_store.inner(), &settings)?;
        match scraper::AiScrapeConfig::from_legacy_settings(
            &settings.ai_api_url,
            &ai_api_key,
            &settings.ai_model,
        ) {
            Ok(config) => Some(config),
            Err(error) => {
                tracing::warn!(error = %error, "AI scrape config rejected; continuing without AI");
                None
            }
        }
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
                r.background = Some(
                    crate::commands::fetch_cover_to_local(url, &format!("{}_bg", r.source_id))
                        .await,
                );
            }
        }
    }

    Ok(ScrapeResponse {
        results,
        source_status,
    })
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
    secret_store: State<'_, SecretStore>,
    queue: State<'_, TaskQueue>,
    query: String,
    source_hint: Option<String>,
    strategy: Option<String>,
) -> Result<Vec<crate::scraper::merge::MergedResult>, String> {
    // Search text can contain personal notes or arbitrary titles, so it never
    // enters the durable job envelope. The provider is route input only; it is
    // likewise not stored unless a future producer supplies a validated ID.
    let job = queue
        .enqueue_operation(
            "合并元数据刮削".to_string(),
            JobOperation::Scrape {
                game_id: "merged_search".to_string(),
                provider_id: None,
            },
            None,
        )
        .map_err(|_| MERGED_SCRAPE_FAILURE.to_string())?;
    let cancellation = queue
        .register_operation(&job.id)
        .map_err(|_| merged_scrape_failed(queue.inner(), &job.id))?;
    queue
        .mark_running(
            &job.id,
            Some("正在查询、合并并增强元数据".to_string()),
            Some(0.05),
        )
        .map_err(|_| merged_scrape_failed(queue.inner(), &job.id))?;

    let outcome = async {
        let settings =
            super::settings::load_settings_with_secret_migration(db.inner(), secret_store.inner())
                .map_err(|_| MERGED_SCRAPE_FAILURE.to_string())?;
        cancellation.check_cancelled()?;

        let strat = match strategy.as_deref() {
            Some("incremental") => scraper::strategy::ScrapeStrategy::Incremental,
            Some("patch_missing") => scraper::strategy::ScrapeStrategy::PatchMissing,
            Some("retry_failed") => scraper::strategy::ScrapeStrategy::RetryFailed,
            _ => scraper::strategy::ScrapeStrategy::Full,
        };

        let proxy = if settings.scraper_proxy.trim().is_empty() {
            None
        } else {
            Some(settings.scraper_proxy.clone())
        };
        scraper::utils::set_proxy(proxy);

        let route = scraper::strategy::ScrapeRouter::plan(source_hint.as_deref(), false, false);
        queue
            .append_event(
                &job.id,
                TaskEventLevel::Info,
                "merged_scrape_sources_started".to_string(),
                "正在查询已启用的数据源".to_string(),
                Some(0.15),
            )
            .map_err(|_| MERGED_SCRAPE_FAILURE.to_string())?;

        let (raw, statuses) = scraper::search_all(
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
        cancellation.check_cancelled()?;
        queue
            .append_event(
                &job.id,
                TaskEventLevel::Info,
                "merged_scrape_sources_completed".to_string(),
                format!("完成 {} 个来源的查询", statuses.len()),
                Some(0.4),
            )
            .map_err(|_| MERGED_SCRAPE_FAILURE.to_string())?;

        let merge_config = scraper::merge::MergeConfig {
            max_results: 10,
            ..Default::default()
        };
        let mut result = scraper::merge::merge_results(raw, &merge_config);
        queue
            .append_event(
                &job.id,
                TaskEventLevel::Info,
                "merged_scrape_results_merged".to_string(),
                format!("已合并为 {} 条候选结果", result.len()),
                Some(0.6),
            )
            .map_err(|_| MERGED_SCRAPE_FAILURE.to_string())?;

        if settings.ai_enabled && route.with_ai {
            cancellation.check_cancelled()?;
            queue
                .append_event(
                    &job.id,
                    TaskEventLevel::Info,
                    "merged_scrape_ai_started".to_string(),
                    "正在补全候选元数据".to_string(),
                    Some(0.65),
                )
                .map_err(|_| MERGED_SCRAPE_FAILURE.to_string())?;

            let ai_api_key = ai_api_key_for_settings(secret_store.inner(), &settings)
                .map_err(|_| MERGED_SCRAPE_FAILURE.to_string())?;
            match scraper::AiScrapeConfig::from_legacy_settings(
                &settings.ai_api_url,
                &ai_api_key,
                &settings.ai_model,
            ) {
                Ok(config) => {
                    let result_count = result.len().max(1);
                    for (index, mr) in result.iter_mut().enumerate() {
                        cancellation.check_cancelled()?;
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
                        cancellation.check_cancelled()?;
                        let progress = 0.65 + ((index + 1) as f64 / result_count as f64) * 0.15;
                        queue
                            .append_event(
                                &job.id,
                                TaskEventLevel::Info,
                                "merged_scrape_ai_progress".to_string(),
                                "正在补全候选元数据".to_string(),
                                Some(progress),
                            )
                            .map_err(|_| MERGED_SCRAPE_FAILURE.to_string())?;
                    }
                }
                Err(_) => {
                    // Never persist configuration details: endpoint strings can
                    // contain credentials. AI enhancement is optional.
                    queue
                        .append_event(
                            &job.id,
                            TaskEventLevel::Warn,
                            "merged_scrape_ai_skipped".to_string(),
                            "AI 补全配置不可用，已继续使用来源结果".to_string(),
                            Some(0.8),
                        )
                        .map_err(|_| MERGED_SCRAPE_FAILURE.to_string())?;
                }
            }
        }

        let asset_count = result.len().max(1);
        for (index, mr) in result.iter_mut().enumerate() {
            cancellation.check_cancelled()?;
            if let Some(ref url) = mr.result.cover {
                if url.starts_with("http") {
                    let local =
                        crate::commands::fetch_cover_to_local(url, &mr.result.source_id).await;
                    mr.result.cover = Some(local);
                }
            }
            cancellation.check_cancelled()?;
            if let Some(ref url) = mr.result.background {
                if url.starts_with("http") {
                    let local = crate::commands::fetch_cover_to_local(
                        url,
                        &format!("{}_bg", mr.result.source_id),
                    )
                    .await;
                    mr.result.background = Some(local);
                }
            }
            cancellation.check_cancelled()?;
            let progress = 0.8 + ((index + 1) as f64 / asset_count as f64) * 0.15;
            queue
                .append_event(
                    &job.id,
                    TaskEventLevel::Info,
                    "merged_scrape_assets_progress".to_string(),
                    "正在缓存候选图片".to_string(),
                    Some(progress),
                )
                .map_err(|_| MERGED_SCRAPE_FAILURE.to_string())?;
        }

        cancellation.check_cancelled()?;
        tracing::info!(results = result.len(), strategy = %strat, "M3 merged scrape completed");
        Ok::<_, String>(result)
    }
    .await;

    match outcome {
        Ok(_result) if cancellation.is_cancelled() => Err("任务已取消".to_string()),
        Ok(result) => match queue.mark_succeeded(
            &job.id,
            Some(format!("完成合并刮削，得到 {} 条候选结果", result.len())),
        ) {
            Ok(_) => Ok(result),
            Err(_) if cancellation.is_cancelled() => Err("任务已取消".to_string()),
            Err(_) => Err(merged_scrape_failed(queue.inner(), &job.id)),
        },
        Err(_) if cancellation.is_cancelled() => Err("任务已取消".to_string()),
        Err(_) => Err(merged_scrape_failed(queue.inner(), &job.id)),
    }
}

const MERGED_SCRAPE_FAILURE: &str = "合并刮削未完成，请检查来源和设置后重试";

fn merged_scrape_failed(queue: &TaskQueue, job_id: &str) -> String {
    let _ = queue.mark_failed(
        job_id,
        ProviderError {
            kind: ProviderErrorKind::Unknown,
            message: MERGED_SCRAPE_FAILURE.to_string(),
            retryable: true,
            retry_after_ms: None,
            provider_id: None,
            operation: Some("scrape".to_string()),
        },
    );
    MERGED_SCRAPE_FAILURE.to_string()
}

fn ai_api_key_for_settings(
    secret_store: &SecretStore,
    settings: &crate::models::Settings,
) -> Result<String, String> {
    let endpoint = scraper::ai_presets::validate_endpoint(&settings.ai_api_url)?;
    if endpoint.is_local() {
        return Ok(String::new());
    }
    secret_store
        .get(SecretKind::AiApiKey, Some(settings.ai_api_url.as_str()))
        .map(|secret| secret.unwrap_or_default())
        .map_err(|error| error.to_string())
}

/// 获取 AI Provider 列表
#[tauri::command]
pub fn get_ai_providers(
    db: State<'_, Database>,
    secret_store: State<'_, SecretStore>,
) -> Result<Vec<scraper::ai_presets::AiProviderDto>, String> {
    let settings =
        super::settings::load_settings_with_secret_migration(db.inner(), secret_store.inner())?;
    let ai_api_key = ai_api_key_for_settings(secret_store.inner(), &settings)?;
    Ok(scraper::ai_presets::provider_dtos_for_settings(
        settings.ai_enabled,
        &settings.ai_api_url,
        &ai_api_key,
        &settings.ai_model,
    ))
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
    secret_store: State<'_, SecretStore>,
    preset_id: String,
    input: String,
) -> Result<String, String> {
    let settings =
        super::settings::load_settings_with_secret_migration(db.inner(), secret_store.inner())?;
    let presets = scraper::ai_presets::builtin_presets();
    let preset = presets
        .iter()
        .find(|p| p.id == preset_id)
        .ok_or_else(|| "AI preset not found".to_string())?;

    if !settings.ai_enabled {
        return Err("AI provider is disabled".to_string());
    }

    // 先按 endpoint canonical origin 解析 Provider，再绑定对应 Key；禁止把默认 Provider
    // 的 Key 无条件复用到用户修改后的其他 origin。
    let ai_api_key = ai_api_key_for_settings(secret_store.inner(), &settings)?;
    let provider = scraper::ai_presets::provider_from_legacy_settings(
        &settings.ai_api_url,
        &ai_api_key,
        &settings.ai_model,
    )?;

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
