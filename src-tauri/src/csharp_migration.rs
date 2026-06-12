// 萌游 MoeGame · C# 旧版数据迁移导入器（M1 ⭐）
//
// 从 C# 原版 MoeGame 导出 JSON 读取游戏库，映射字段后写入 SQLite。
// 支持：
//   - 封面/背景文件复制到新数据目录
//   - Notes 里 `<!--moe:cn name=… desc=…-->` 中文标记解析
//   - 幂等可重入（按 id upsert）
//   - 进度事件推送到前端
//   - 迁移前后完整性校验

use crate::db_sqlite::SqliteDb;
use crate::models::{Game, GameAlias, GameMetadata, PlayTracker, SaveData};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// ============================================================================
// C# 导出格式（Playnite Game 模型 + MoeGame 扩展）
// ============================================================================

/// C# 导出的游戏 JSON 结构（宽松反序列化：未知字段忽略）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CSharpGame {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub sort_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub cover_image: Option<String>,
    #[serde(default)]
    pub background_image: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub install_directory: Option<String>,
    #[serde(default)]
    pub game_image_path: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub plugin_id: Option<String>,
    #[serde(default)]
    pub game_id: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<CSharpTag>>,
    #[serde(default)]
    pub genres: Option<Vec<String>>,
    #[serde(default)]
    pub developers: Option<Vec<String>>,
    #[serde(default)]
    pub publishers: Option<Vec<String>>,
    #[serde(default)]
    pub release_date: Option<String>,
    #[serde(default)]
    pub release_year: Option<u32>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub series: Option<String>,
    #[serde(default)]
    pub age_rating: Option<String>,
    #[serde(default)]
    pub community_score: Option<f64>,
    #[serde(default)]
    pub playtime: Option<u64>,
    #[serde(default)]
    pub last_activity: Option<String>,
    #[serde(default)]
    pub completion_status: Option<String>,
    #[serde(default)]
    pub favorite: Option<bool>,
    #[serde(default)]
    pub hidden: Option<bool>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub added: Option<String>,
    #[serde(default)]
    pub modified: Option<String>,
    // MoeGame 扩展
    #[serde(default)]
    pub original_name: Option<String>,
    #[serde(default)]
    pub name_cn: Option<String>,
    #[serde(default)]
    pub description_cn: Option<String>,
    #[serde(default)]
    pub engine: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub vndb_id: Option<String>,
    #[serde(default)]
    pub bangumi_id: Option<String>,
    #[serde(default)]
    pub vndb_rating: Option<f64>,
    #[serde(default)]
    pub bangumi_rating: Option<f64>,
    #[serde(default)]
    pub estimated_hours: Option<f64>,
    #[serde(default)]
    pub install_dir_locale_jp: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CSharpTag {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
}

/// C# 导出文件的顶层结构。
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum CSharpExport {
    /// 直接的游戏数组
    GamesArray(Vec<CSharpGame>),
    /// 带包装的对象
    Wrapped {
        #[serde(default)]
        #[serde(alias = "Games")]
        games: Vec<CSharpGame>,
    },
}

struct MigrationInput {
    json_path: PathBuf,
    media_root: Option<PathBuf>,
    source_label: String,
}

enum DiscoveredMigrationInput {
    LiteDb {
        library_dir: PathBuf,
        litedb_dll: PathBuf,
    },
    Json {
        json_path: PathBuf,
        media_root: Option<PathBuf>,
    },
}

const LITEDB_EXPORT_SCRIPT: &str = r#"
param(
  [Parameter(Mandatory=$true)][string]$LibraryDir,
  [Parameter(Mandatory=$true)][string]$LiteDbDll,
  [Parameter(Mandatory=$true)][string]$OutPath
)

$ErrorActionPreference = 'Stop'
Add-Type -Path $LiteDbDll

function Open-LiteDb([string]$Path) {
  [LiteDB.LiteDatabase]::new("Filename=$Path;Mode=ReadOnly")
}

function Has($Doc, [string]$Key) {
  return $null -ne $Doc -and $Doc.ContainsKey($Key) -and -not $Doc[$Key].IsNull
}

function String-Value($Doc, [string]$Key) {
  if (Has $Doc $Key) { return $Doc[$Key].AsString }
  return $null
}

function Guid-Value($Value) {
  if ($null -eq $Value -or $Value.IsNull) { return $null }
  if ($Value.IsGuid) { return $Value.AsGuid.ToString() }
  if ($null -ne $Value.RawValue) { return [string]$Value.RawValue }
  return $Value.ToString()
}

function Ids-Value($Doc, [string]$Key) {
  $ids = @()
  if (Has $Doc $Key) {
    foreach ($item in $Doc[$Key].AsArray) {
      $id = Guid-Value $item
      if ($id) { $ids += $id }
    }
  }
  return $ids
}

function Date-Value($Doc, [string]$Key) {
  if (Has $Doc $Key -and $Doc[$Key].IsDateTime) {
    return $Doc[$Key].AsDateTime.ToString('yyyy-MM-dd HH:mm')
  }
  return $null
}

function Number-Value($Doc, [string]$Key) {
  if (-not (Has $Doc $Key)) { return $null }
  $raw = $Doc[$Key].RawValue
  if ($null -eq $raw) { return $null }
  return [double]$raw
}

function Long-Value($Doc, [string]$Key) {
  if (-not (Has $Doc $Key)) { return $null }
  $raw = $Doc[$Key].RawValue
  if ($null -eq $raw) { return $null }
  return [int64]$raw
}

function Bool-Value($Doc, [string]$Key) {
  if (Has $Doc $Key) { return [bool]$Doc[$Key].AsBoolean }
  return $null
}

function Media-Path([string]$Value) {
  if ([string]::IsNullOrWhiteSpace($Value)) { return $null }
  if ($Value.StartsWith('http://') -or $Value.StartsWith('https://')) { return $Value }
  if ([System.IO.Path]::IsPathRooted($Value)) { return $Value }
  return (Join-Path (Join-Path $LibraryDir 'files') $Value)
}

function Load-Lookup([string]$DbName, [string]$CollectionName) {
  $map = @{}
  $path = Join-Path $LibraryDir $DbName
  if (-not (Test-Path $path)) { return $map }
  $db = Open-LiteDb $path
  try {
    foreach ($doc in $db.GetCollection($CollectionName).FindAll()) {
      $id = Guid-Value $doc['_id']
      $name = String-Value $doc 'Name'
      if ($id -and $name) { $map[$id] = $name }
    }
  } finally {
    $db.Dispose()
  }
  return $map
}

function Resolve-Ids($Ids, $Lookup) {
  $names = @()
  foreach ($id in $Ids) {
    if ($Lookup.ContainsKey($id)) { $names += $Lookup[$id] }
  }
  return $names
}

function Resolve-One($Id, $Lookup) {
  if ($Id -and $Lookup.ContainsKey($Id)) { return $Lookup[$Id] }
  return $null
}

$genres = Load-Lookup 'genres.db' 'Genre'
$companies = Load-Lookup 'companies.db' 'Company'
$sources = Load-Lookup 'sources.db' 'GameSource'
$platforms = Load-Lookup 'platforms.db' 'Platform'
$ageRatings = Load-Lookup 'ageratings.db' 'AgeRating'
$series = Load-Lookup 'series.db' 'Series'
$tags = Load-Lookup 'tags.db' 'Tag'

$gamesDbPath = Join-Path $LibraryDir 'games.db'
$db = Open-LiteDb $gamesDbPath
$games = @()
try {
  foreach ($doc in $db.GetCollection('Game').FindAll()) {
    $completion = $null
    if (Has $doc 'CompletionStatus') {
      $completion = String-Value ($doc['CompletionStatus'].AsDocument) 'Name'
    }
    $sourceId = Guid-Value $doc['SourceId']
    $sourceName = Resolve-One $sourceId $sources
    $platformIds = @(Ids-Value $doc 'PlatformIds')
    $developerIds = @(Ids-Value $doc 'DeveloperIds')
    $publisherIds = @(Ids-Value $doc 'PublisherIds')
    $genreIds = @(Ids-Value $doc 'GenreIds')
    $tagIds = @(Ids-Value $doc 'TagIds')
    $platformNames = @(Resolve-Ids $platformIds $platforms)
    $developerNames = @(Resolve-Ids $developerIds $companies)
    $publisherNames = @(Resolve-Ids $publisherIds $companies)
    $genreNames = @(Resolve-Ids $genreIds $genres)
    $tagNames = @(Resolve-Ids $tagIds $tags)
    $ageId = @(Ids-Value $doc 'AgeRatingIds') | Select-Object -First 1
    $seriesId = @(Ids-Value $doc 'SeriesIds') | Select-Object -First 1
    $ageName = Resolve-One $ageId $ageRatings
    $seriesName = Resolve-One $seriesId $series
    $links = @()
    if (Has $doc 'Links') {
      foreach ($link in $doc['Links'].AsArray) {
        $ld = $link.AsDocument
        $links += [pscustomobject]@{
          Name = String-Value $ld 'Name'
          Url = String-Value $ld 'Url'
        }
      }
    }
    $homepage = $null
    foreach ($link in $links) {
      if ($link.Url -and ($link.Name -eq 'Official Website' -or -not $homepage)) {
        $homepage = $link.Url
      }
    }

    $gameId = String-Value $doc 'GameId'
    $imagePath = $null
    if ($sourceName -eq 'Steam' -and $gameId) {
      $imagePath = "steam://rungameid/$gameId"
    }

    $games += [pscustomobject]@{
      Id = (Guid-Value $doc['_id'])
      Name = (String-Value $doc 'Name')
      Description = (String-Value $doc 'Description')
      CoverImage = (Media-Path (String-Value $doc 'CoverImage'))
      BackgroundImage = (Media-Path (String-Value $doc 'BackgroundImage'))
      Icon = (Media-Path (String-Value $doc 'Icon'))
      InstallDirectory = (String-Value $doc 'InstallDirectory')
      GameImagePath = $imagePath
      Source = $sourceName
      PluginId = (Guid-Value $doc['PluginId'])
      GameId = $gameId
      Tags = @($tagNames | ForEach-Object { [pscustomobject]@{ Name = $_; Category = 'tag' } })
      Genres = @($genreNames)
      Developers = @($developerNames)
      Publishers = @($publisherNames)
      ReleaseDate = (String-Value $doc 'ReleaseDate')
      AgeRating = $ageName
      Series = $seriesName
      CommunityScore = (Number-Value $doc 'CommunityScore')
      Playtime = (Long-Value $doc 'Playtime')
      LastActivity = (Date-Value $doc 'LastActivity')
      CompletionStatus = $completion
      Favorite = (Bool-Value $doc 'Favorite')
      Hidden = (Bool-Value $doc 'Hidden')
      Notes = (String-Value $doc 'Notes')
      Added = (Date-Value $doc 'Added')
      Modified = (Date-Value $doc 'Modified')
      Homepage = $homepage
      Platform = ($platformNames -join ', ')
    }
  }
} finally {
  $db.Dispose()
}

$payload = [ordered]@{
  games = $games
  source = 'PlayniteLiteDB'
  library_dir = $LibraryDir
  exported_at = (Get-Date).ToString('o')
}

$parent = Split-Path -Parent $OutPath
if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
[System.IO.File]::WriteAllText($OutPath, ($payload | ConvertTo-Json -Depth 10), [System.Text.UTF8Encoding]::new($false))
"Exported $($games.Count) games to $OutPath"
"#;

// ============================================================================
// 迁移报告
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationReport {
    pub total_found: usize,
    pub imported: usize,
    pub updated: usize,
    pub skipped: usize,
    pub media_copied: usize,
    pub media_missing: usize,
    pub errors: Vec<String>,
    pub duration_secs: f64,
    pub source_label: String,
    pub source_ids: Vec<String>,
    pub backup_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationProgress {
    pub current: usize,
    pub total: usize,
    pub stage: String,
    pub message: String,
}

// ============================================================================
// 主入口
// ============================================================================

/// 从 C# 导出的 JSON 文件迁移数据到 SQLite。
///
/// `source_path` 指向 C# 导出的 JSON 文件，或 C# 数据目录（自动找 JSON）。
/// `db` 为目标 SQLite 数据库。
/// `data_dir` 为新数据目录根（媒体文件复制到这里）。
/// `on_progress` 进度回调。
pub fn migrate_from_csharp(
    source_path: &Path,
    db: &SqliteDb,
    data_dir: &Path,
    on_progress: &dyn Fn(MigrationProgress),
) -> Result<MigrationReport, String> {
    let start = std::time::Instant::now();

    // 1. 找到 JSON 文件或旧 Playnite LiteDB 库
    on_progress(MigrationProgress {
        current: 0,
        total: 0,
        stage: "discover".into(),
        message: "正在搜索 C# 导出文件或 Playnite LiteDB 库...".into(),
    });

    let discovered = discover_migration_input(source_path)?;
    let backup_dir = backup_existing_database(data_dir)?;
    let input = prepare_migration_input(discovered, data_dir)?;
    tracing::info!(
        path = %input.json_path.display(),
        source = %input.source_label,
        "Found migration input"
    );

    // 2. 读取并解析
    on_progress(MigrationProgress {
        current: 0,
        total: 0,
        stage: "parse".into(),
        message: format!("正在解析 {} 游戏数据...", input.source_label),
    });

    let content =
        fs::read_to_string(&input.json_path).map_err(|e| format!("读取 JSON 文件失败: {}", e))?;
    let content = content.trim_start_matches('\u{feff}');
    let export: CSharpExport =
        serde_json::from_str(content).map_err(|e| format!("解析 JSON 失败: {}", e))?;
    let csharp_games = match export {
        CSharpExport::GamesArray(games) => games,
        CSharpExport::Wrapped { games } => games,
    };

    let total = csharp_games.len();
    let source_ids = csharp_games
        .iter()
        .map(|game| game.id.clone())
        .collect::<Vec<_>>();
    tracing::info!(total, "Parsed C# games");

    // 3. 确保媒体目录存在
    let covers_dir = data_dir.join("covers");
    let backgrounds_dir = data_dir.join("backgrounds");
    let icons_dir = data_dir.join("icons");
    fs::create_dir_all(&covers_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&backgrounds_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&icons_dir).map_err(|e| e.to_string())?;

    // 4. 逐游戏映射 + 导入
    let mut imported = 0usize;
    let mut updated = 0usize;
    let mut skipped = 0usize;
    let mut media_copied = 0usize;
    let mut media_missing = 0usize;
    let mut errors: Vec<String> = Vec::new();
    let mut games_to_import: Vec<Game> = Vec::with_capacity(total);

    for (i, cg) in csharp_games.iter().enumerate() {
        on_progress(MigrationProgress {
            current: i + 1,
            total,
            stage: "migrate".into(),
            message: format!("正在迁移: {}", cg.name),
        });

        match map_csharp_to_game(
            cg,
            input.media_root.as_deref(),
            &covers_dir,
            &backgrounds_dir,
            &icons_dir,
        ) {
            Ok((game, copied, missing)) => {
                // 检查是否已存在
                let exists = db.get_game_opt(&game.id).unwrap_or(None).is_some();
                games_to_import.push(game);
                if exists {
                    updated += 1;
                } else {
                    imported += 1;
                }
                media_copied += copied;
                media_missing += missing;
            }
            Err(e) => {
                skipped += 1;
                let err_msg = format!("{} (ID: {}): {}", cg.name, cg.id, e);
                tracing::warn!("{}", err_msg);
                errors.push(err_msg);
            }
        }

        // 每 50 个游戏批量写入一次
        if games_to_import.len() >= 50 {
            db.import_games(&games_to_import)?;
            games_to_import.clear();
        }
    }

    // 写入剩余
    if !games_to_import.is_empty() {
        db.import_games(&games_to_import)?;
    }

    let duration_secs = start.elapsed().as_secs_f64();

    on_progress(MigrationProgress {
        current: total,
        total,
        stage: "done".into(),
        message: format!(
            "迁移完成：导入 {} / 更新 {} / 跳过 {} / 媒体 {} 复制 {} 缺失",
            imported, updated, skipped, media_copied, media_missing
        ),
    });

    Ok(MigrationReport {
        total_found: total,
        imported,
        updated,
        skipped,
        media_copied,
        media_missing,
        errors,
        duration_secs,
        source_label: input.source_label,
        source_ids,
        backup_dir: backup_dir.map(|path| path.display().to_string()),
    })
}

// ============================================================================
// 字段映射
// ============================================================================

/// 将 C# 游戏映射为我们的 Game 模型。
/// 返回 (Game, 已复制媒体数, 媒体缺失数)。
fn map_csharp_to_game(
    cg: &CSharpGame,
    media_root: Option<&Path>,
    covers_dir: &Path,
    backgrounds_dir: &Path,
    icons_dir: &Path,
) -> Result<(Game, usize, usize), String> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string();
    let created = cg.added.clone().unwrap_or_else(|| now.clone());
    let updated = cg.modified.clone().unwrap_or_else(|| now.clone());

    let mut game = Game {
        id: cg.id.clone(),
        name: cg.name.clone(),
        exe_path: cg
            .game_image_path
            .clone()
            .unwrap_or_else(|| cg.install_directory.clone().unwrap_or_default()),
        install_dir: cg.install_directory.clone(),
        game_type: cg.source.clone(),
        library_source: None,
        library_id: None,
        launch_uri: None,
        last_imported_at: None,
        created_at: created,
        updated_at: updated,
        description: cg.description.clone(),
        cover: None,
        background: None,
        icon: None,
        screenshots: vec![],
        favorite: cg.favorite.unwrap_or(false),
        hidden: cg.hidden.unwrap_or(false),
        tags: cg
            .tags
            .as_ref()
            .map(|t| t.iter().filter_map(|t| t.name.clone()).collect())
            .unwrap_or_default(),
        metadata: GameMetadata {
            developer: cg.developers.as_ref().and_then(|d| d.first().cloned()),
            publisher: cg.publishers.as_ref().and_then(|p| p.first().cloned()),
            platform: parse_game_platform(cg.platform.as_deref()),
            engine: cg.engine.clone(),
            genres: cg.genres.clone().unwrap_or_default(),
            languages: vec![],
            voice_languages: vec![],
            version: cg.version.clone(),
            original_name: cg.original_name.clone(),
            homepage: cg.homepage.clone(),
            developer_homepage: None,
            stores: vec![],
            age_rating: cg.age_rating.clone(),
            series: cg.series.clone(),
            release_date: cg.release_date.clone(),
            release_year: cg.release_year,
            estimated_hours: cg.estimated_hours,
            vndb_rating: cg.vndb_rating,
            bangumi_rating: cg.bangumi_rating,
            vndb_id: cg.vndb_id.clone(),
            bangumi_id: cg.bangumi_id.clone(),
            cover: None,
            background: None,
        },
        play_tracker: PlayTracker {
            total_seconds: cg.playtime.unwrap_or(0),
            sessions: vec![],
            completion_status: parse_completion_status(cg.completion_status.as_deref()),
            last_played: cg.last_activity.clone(),
            first_played: None,
            user_rating: cg.community_score,
            review: None,
            achievements_total: 0,
            achievements_unlocked: 0,
            finished: cg
                .completion_status
                .as_deref()
                .map(|s| s.to_lowercase() == "completed")
                .unwrap_or(false),
            completion_count: 0,
        },
        save_data: SaveData::default(),
        aliases: vec![],
        tag_entries: vec![],
        release_year: cg.release_year,
        rating: cg.community_score,
        last_played: cg.last_activity.clone(),
        vndb_id: cg.vndb_id.clone(),
        bangumi_id: cg.bangumi_id.clone(),
        play_time_seconds: cg.playtime.unwrap_or(0),
        add_date: cg.added.clone(),
    };

    // 解析 Notes 里的中文标记
    if let Some(ref notes) = cg.notes {
        let (name_cn, desc_cn) = parse_chinese_markers(notes);
        if let Some(cn) = name_cn {
            game.aliases.push(GameAlias {
                name: cn.clone(),
                language: Some("zh".into()),
                source: Some("migration".into()),
                is_primary: false,
            });
            // 如果 C# 侧有 name_cn 字段，优先用
            if cg.name_cn.is_none() {
                // 暂存到 aliases 里
            }
        }
        if let Some(ref cn) = desc_cn {
            if game.description.is_none() {
                game.description = Some(cn.clone());
            }
        }
    }

    // C# 侧显式的 cn 字段优先
    if let Some(ref name_cn) = cg.name_cn {
        if !game.aliases.iter().any(|a| a.name == *name_cn) {
            game.aliases.push(GameAlias {
                name: name_cn.clone(),
                language: Some("zh".into()),
                source: Some("migration".into()),
                is_primary: true,
            });
        }
    }
    if let Some(ref desc_cn) = cg.description_cn {
        game.description = Some(desc_cn.clone());
    }

    apply_platform_identity(cg, &mut game);

    // 复制媒体文件
    let (copied, missing) = copy_media(
        cg,
        &mut game,
        media_root,
        covers_dir,
        backgrounds_dir,
        icons_dir,
    )?;

    Ok((game, copied, missing))
}

// ============================================================================
// 媒体文件复制
// ============================================================================

fn copy_media(
    cg: &CSharpGame,
    game: &mut Game,
    media_root: Option<&Path>,
    covers_dir: &Path,
    backgrounds_dir: &Path,
    icons_dir: &Path,
) -> Result<(usize, usize), String> {
    let mut copied = 0usize;
    let mut missing = 0usize;

    // 封面
    if let Some(ref cover_path) = cg.cover_image {
        if let Some(new_path) =
            copy_if_exists(cover_path, media_root, covers_dir, &game.id, "cover")
        {
            game.cover = Some(new_path);
            game.metadata.cover = game.cover.clone();
            copied += 1;
        } else {
            missing += 1;
        }
    }

    // 背景
    if let Some(ref bg_path) = cg.background_image {
        if let Some(new_path) = copy_if_exists(bg_path, media_root, backgrounds_dir, &game.id, "bg")
        {
            game.background = Some(new_path);
            game.metadata.background = game.background.clone();
            copied += 1;
        } else {
            missing += 1;
        }
    }

    // 图标
    if let Some(ref icon_path) = cg.icon {
        if let Some(new_path) = copy_if_exists(icon_path, media_root, icons_dir, &game.id, "icon") {
            game.icon = Some(new_path);
            copied += 1;
        } else {
            missing += 1;
        }
    }

    Ok((copied, missing))
}

fn copy_if_exists(
    src: &str,
    media_root: Option<&Path>,
    dest_dir: &Path,
    game_id: &str,
    prefix: &str,
) -> Option<String> {
    let src = src.trim();
    if src.is_empty() {
        return None;
    }
    // 跳过 URL（保留原值）
    if src.starts_with("http") {
        return Some(src.to_string());
    }
    let src_path = resolve_media_path(src, media_root);
    if !src_path.is_file() {
        return None;
    }
    let ext = src_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg");
    let dest_name = format!("{}_{}.{}", sanitize_file_stem(game_id), prefix, ext);
    let dest_path = dest_dir.join(&dest_name);
    match fs::copy(src_path, &dest_path) {
        Ok(_) => Some(dest_path.to_string_lossy().to_string()),
        Err(e) => {
            tracing::warn!(src, dest = %dest_path.display(), error = %e, "Failed to copy media");
            None
        }
    }
}

fn resolve_media_path(src: &str, media_root: Option<&Path>) -> PathBuf {
    let src_path = Path::new(src);
    if src_path.is_absolute() {
        src_path.to_path_buf()
    } else if let Some(root) = media_root {
        root.join(src_path)
    } else {
        src_path.to_path_buf()
    }
}

fn sanitize_file_stem(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => ch,
        })
        .collect()
}

// ============================================================================
// 中文标记解析
// ============================================================================

/// 解析 Notes 里的 `<!--moe:cn name=… desc=…-->` 标记。
fn parse_chinese_markers(notes: &str) -> (Option<String>, Option<String>) {
    // 匹配整个 <!--moe:cn ... --> 块
    let block_re = regex::Regex::new(r"<!--moe:cn\s+(.*?)-->").ok();
    if let Some(re) = block_re {
        if let Some(caps) = re.captures(notes) {
            let inner = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            // 去掉末尾可能残留的 `>`
            let inner = inner.trim_end_matches('>').trim();
            let name = extract_attr(inner, "name");
            let desc = extract_attr(inner, "desc");
            return (name, desc);
        }
    }
    (None, None)
}

/// 从 key="value" 或 key=value 格式中提取值。
fn extract_attr(s: &str, key: &str) -> Option<String> {
    // 匹配 key="value" 或 key='value' 或 key=value（不含空格）
    let pat = format!(r#"{}=(?:"([^"]*)"|'([^']*)'|(\S+))"#, regex::escape(key));
    let re = regex::Regex::new(&pat).ok()?;
    if let Some(caps) = re.captures(s) {
        // 优先双引号，其次单引号，最后无引号
        if let Some(v) = caps.get(1) {
            return Some(v.as_str().to_string());
        }
        if let Some(v) = caps.get(2) {
            return Some(v.as_str().to_string());
        }
        if let Some(v) = caps.get(3) {
            return Some(v.as_str().to_string());
        }
    }
    None
}

// ============================================================================
// 工具函数
// ============================================================================

fn discover_migration_input(source_path: &Path) -> Result<DiscoveredMigrationInput, String> {
    if let Some(library_dir) = find_playnite_library_dir(source_path) {
        let litedb_dll = find_litedb_dll(source_path, &library_dir)?;
        return Ok(DiscoveredMigrationInput::LiteDb {
            library_dir,
            litedb_dll,
        });
    }

    let json_path = find_export_json(source_path)?;
    let media_root = infer_media_root_for_json(&json_path, source_path);
    Ok(DiscoveredMigrationInput::Json {
        json_path,
        media_root,
    })
}

fn prepare_migration_input(
    discovered: DiscoveredMigrationInput,
    data_dir: &Path,
) -> Result<MigrationInput, String> {
    match discovered {
        DiscoveredMigrationInput::LiteDb {
            library_dir,
            litedb_dll,
        } => {
            let export_path = export_litedb_to_json(&library_dir, &litedb_dll, data_dir)?;
            Ok(MigrationInput {
                json_path: export_path,
                media_root: Some(library_dir.join("files")),
                source_label: format!("Playnite LiteDB ({})", library_dir.display()),
            })
        }
        DiscoveredMigrationInput::Json {
            json_path,
            media_root,
        } => Ok(MigrationInput {
            json_path,
            media_root,
            source_label: "C# JSON".into(),
        }),
    }
}

fn find_export_json(source_path: &Path) -> Result<PathBuf, String> {
    if source_path.is_file() {
        return Ok(source_path.to_path_buf());
    }
    if source_path.is_dir() {
        // 搜索目录下的 JSON 导出文件
        for name in &["export.json", "library_export.json", "moegame_export.json"] {
            let candidate = source_path.join(name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
        let database_json = source_path.join("database.json");
        if database_json.is_file() && looks_like_csharp_export(&database_json) {
            return Ok(database_json);
        }
        // 搜索任意 .json 文件
        if let Ok(entries) = fs::read_dir(source_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    // 快速检测是否是游戏库导出
                    if let Ok(content) = fs::read_to_string(&path) {
                        if content.contains("\"Name\"") && content.contains("\"Id\"") {
                            return Ok(path);
                        }
                    }
                }
            }
        }
    }
    Err(format!(
        "未找到 C# 导出文件。请将 C# 版游戏库导出为 JSON 放到: {}",
        source_path.display()
    ))
}

fn looks_like_csharp_export(path: &Path) -> bool {
    fs::read_to_string(path)
        .map(|content| content.contains("\"Name\"") && content.contains("\"Id\""))
        .unwrap_or(false)
}

fn find_playnite_library_dir(source_path: &Path) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if source_path.is_file() {
        if source_path.file_name().and_then(|n| n.to_str()) == Some("games.db") {
            return source_path.parent().map(Path::to_path_buf);
        }
        if let Some(parent) = source_path.parent() {
            candidates.push(parent.to_path_buf());
        }
    } else if source_path.is_dir() {
        candidates.push(source_path.to_path_buf());
        candidates.push(source_path.join("library"));
    }
    if let Some(local_dir) = dirs::data_local_dir() {
        candidates.push(local_dir.join("MoeGameSetup").join("library"));
        candidates.push(local_dir.join("Playnite").join("library"));
    }
    if let Some(data_dir) = dirs::data_dir() {
        candidates.push(data_dir.join("Playnite").join("library"));
    }

    candidates
        .into_iter()
        .find(|dir| dir.join("games.db").is_file())
}

fn find_litedb_dll(source_path: &Path, library_dir: &Path) -> Result<PathBuf, String> {
    let mut candidates = Vec::new();
    if let Some(parent) = library_dir.parent() {
        candidates.push(parent.join("LiteDB.dll"));
        candidates.push(
            parent
                .join("source")
                .join("packages")
                .join("LiteDB.4.1.4")
                .join("lib")
                .join("net45")
                .join("LiteDB.dll"),
        );
        candidates.push(
            parent
                .join("source")
                .join("packages")
                .join("LiteDB.4.1.4")
                .join("lib")
                .join("net40")
                .join("LiteDB.dll"),
        );
    }
    if source_path.is_dir() {
        candidates.push(source_path.join("LiteDB.dll"));
        candidates.push(source_path.join("..").join("LiteDB.dll"));
    }
    if let Some(local_dir) = dirs::data_local_dir() {
        candidates.push(local_dir.join("MoeGameSetup").join("LiteDB.dll"));
        candidates.push(local_dir.join("Playnite").join("LiteDB.dll"));
    }
    if let Some(home_dir) = dirs::home_dir() {
        candidates.push(
            home_dir
                .join(".nuget")
                .join("packages")
                .join("litedb")
                .join("4.1.4")
                .join("lib")
                .join("net45")
                .join("LiteDB.dll"),
        );
        candidates.push(
            home_dir
                .join(".nuget")
                .join("packages")
                .join("litedb")
                .join("4.1.4")
                .join("lib")
                .join("net40")
                .join("LiteDB.dll"),
        );
        candidates.push(
            home_dir
                .join(".nuget")
                .join("packages")
                .join("litedb")
                .join("4.1.4")
                .join("lib")
                .join("net35")
                .join("LiteDB.dll"),
        );
        candidates.push(
            home_dir
                .join("Desktop")
                .join("开发")
                .join("moeplay")
                .join("dist")
                .join("MoeGame-Portable")
                .join("LiteDB.dll"),
        );
        candidates.push(
            home_dir
                .join("Desktop")
                .join("开发")
                .join("moeplay")
                .join("source")
                .join("packages")
                .join("LiteDB.4.1.4")
                .join("lib")
                .join("net40")
                .join("LiteDB.dll"),
        );
        candidates.push(
            home_dir
                .join("Desktop")
                .join("开发")
                .join("moeplay")
                .join("source")
                .join("packages")
                .join("LiteDB.4.1.4")
                .join("lib")
                .join("net35")
                .join("LiteDB.dll"),
        );
    }
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| {
            format!(
                "找到 Playnite 库 {}，但未找到 LiteDB.dll",
                library_dir.display()
            )
        })
}

fn infer_media_root_for_json(json_path: &Path, source_path: &Path) -> Option<PathBuf> {
    let base = if source_path.is_dir() {
        source_path.to_path_buf()
    } else {
        json_path.parent().unwrap_or(source_path).to_path_buf()
    };
    [base.join("library").join("files"), base.join("files")]
        .into_iter()
        .find(|candidate| candidate.is_dir())
}

fn export_litedb_to_json(
    library_dir: &Path,
    litedb_dll: &Path,
    data_dir: &Path,
) -> Result<PathBuf, String> {
    let migration_dir = data_dir.join("migration");
    fs::create_dir_all(&migration_dir).map_err(|e| e.to_string())?;
    let script_path = migration_dir.join("export_playnite_litedb.ps1");
    let out_path = migration_dir.join("library_export.json");
    fs::write(&script_path, LITEDB_EXPORT_SCRIPT).map_err(|e| e.to_string())?;

    let output = Command::new("powershell")
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(&script_path)
        .arg("-LibraryDir")
        .arg(library_dir)
        .arg("-LiteDbDll")
        .arg(litedb_dll)
        .arg("-OutPath")
        .arg(&out_path)
        .output()
        .map_err(|e| format!("启动 LiteDB 导出脚本失败: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "LiteDB 导出失败: {}\n{}",
            stderr.trim(),
            stdout.trim()
        ));
    }
    if !out_path.is_file() {
        return Err("LiteDB 导出未生成 library_export.json".into());
    }
    Ok(out_path)
}

fn backup_existing_database(data_dir: &Path) -> Result<Option<PathBuf>, String> {
    let targets = [
        "moegame.db",
        "moegame.db-wal",
        "moegame.db-shm",
        "database.json",
        "manifest.json",
    ];
    if !targets.iter().any(|name| data_dir.join(name).is_file()) {
        return Ok(None);
    }
    let backup_dir = data_dir.join("backups");
    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;
    let stamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
    let snapshot_dir = backup_dir.join(format!("pre-migration-{}", stamp));
    fs::create_dir_all(&snapshot_dir)
        .map_err(|e| format!("create migration backup failed: {e}"))?;
    for name in targets {
        let source = data_dir.join(name);
        if source.is_file() {
            fs::copy(&source, snapshot_dir.join(name))
                .map(|_| ())
                .map_err(|e| format!("backup {name} before migration failed: {e}"))?;
        }
    }

    Ok(Some(snapshot_dir))
}

fn apply_platform_identity(cg: &CSharpGame, game: &mut Game) {
    let source = cg.source.as_deref().unwrap_or_default().to_lowercase();
    if source == "steam" {
        if let Some(app_id) = cg.game_id.as_ref().filter(|id| !id.trim().is_empty()) {
            let uri = format!("steam://rungameid/{}", app_id.trim());
            game.library_source = Some("steam".into());
            game.library_id = Some(app_id.trim().into());
            game.launch_uri = Some(uri.clone());
            if game.exe_path.trim().is_empty() {
                game.exe_path = uri;
            }
            game.metadata.platform = Some(crate::models::GamePlatform::PC);
        }
    }
}

fn parse_game_platform(platform: Option<&str>) -> Option<crate::models::GamePlatform> {
    let platform = platform?.to_lowercase();
    if platform.contains("windows") || platform.contains("pc") || platform.contains("steam") {
        Some(crate::models::GamePlatform::PC)
    } else if platform.contains("web") || platform.contains("browser") {
        Some(crate::models::GamePlatform::Web)
    } else if platform.contains("android")
        || platform.contains("ios")
        || platform.contains("mobile")
        || platform.contains("phone")
    {
        Some(crate::models::GamePlatform::Mobile)
    } else if platform.contains("switch")
        || platform.contains("playstation")
        || platform.contains("xbox")
        || platform.contains("console")
    {
        Some(crate::models::GamePlatform::Console)
    } else if platform.trim().is_empty() {
        None
    } else {
        Some(crate::models::GamePlatform::Other)
    }
}

fn parse_completion_status(s: Option<&str>) -> crate::models::CompletionStatus {
    use crate::models::CompletionStatus;
    match s.map(|s| s.to_lowercase()).as_deref() {
        Some("notstarted") | Some("not_started") | None => CompletionStatus::NotStarted,
        Some("playing") => CompletionStatus::Playing,
        Some("completed") => CompletionStatus::Completed,
        Some("dropped") => CompletionStatus::Dropped,
        Some("onhold") | Some("on_hold") => CompletionStatus::OnHold,
        Some("plantoplay") | Some("plan_to_play") => CompletionStatus::PlanToPlay,
        Some("replaying") => CompletionStatus::Replaying,
        _ => CompletionStatus::NotStarted,
    }
}

// ============================================================================
// 完整性校验
// ============================================================================

/// 迁移后校验：对比游戏数、封面存在率、抽样字段。
pub fn verify_migration(
    db: &SqliteDb,
    expected_count: usize,
) -> Result<MigrationVerifyReport, String> {
    verify_migration_for_ids(db, expected_count, &[])
}

pub fn verify_migration_for_ids(
    db: &SqliteDb,
    expected_count: usize,
    source_ids: &[String],
) -> Result<MigrationVerifyReport, String> {
    let actual_count = db.game_count().map_err(|e| e.to_string())? as usize;
    let games = db.list_games()?;
    let games_by_id = games
        .iter()
        .map(|game| (game.id.as_str(), game))
        .collect::<HashMap<_, _>>();
    let source_id_set = source_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let relevant_games = if source_id_set.is_empty() {
        games.iter().collect::<Vec<_>>()
    } else {
        games
            .iter()
            .filter(|game| source_id_set.contains(game.id.as_str()))
            .collect::<Vec<_>>()
    };
    let all_missing_ids = source_ids
        .iter()
        .filter(|id| !games_by_id.contains_key(id.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let missing_ids = all_missing_ids.iter().take(20).cloned().collect::<Vec<_>>();
    let matched_count = if source_id_set.is_empty() {
        actual_count
    } else {
        source_ids.len().saturating_sub(all_missing_ids.len())
    };
    let missing_count = if source_id_set.is_empty() {
        expected_count.saturating_sub(actual_count)
    } else {
        all_missing_ids.len()
    };

    let with_cover = relevant_games.iter().filter(|g| g.cover.is_some()).count();
    let with_bg = relevant_games
        .iter()
        .filter(|g| g.background.is_some())
        .count();
    let with_desc = relevant_games
        .iter()
        .filter(|g| g.description.is_some())
        .count();
    let cover_rate = if expected_count > 0 {
        with_cover as f64 / expected_count as f64
    } else {
        0.0
    };
    let mut issues = Vec::new();
    let count_match = missing_count == 0 && actual_count >= expected_count;
    if !count_match {
        issues.push(format!(
            "迁移数量不一致：期望 {}，匹配 {}，库内总数 {}",
            expected_count, matched_count, actual_count
        ));
    }
    if expected_count > 0 && cover_rate < 0.95 {
        issues.push(format!("封面覆盖率偏低：{:.1}%", cover_rate * 100.0));
    }
    if matched_count > 0 && with_bg == 0 {
        issues.push("没有检测到背景图，请确认旧版 library/files 是否可访问".into());
    }
    if missing_count > 0 && !missing_ids.is_empty() {
        issues.push(format!("缺失游戏 ID 示例：{}", missing_ids.join(", ")));
    }

    Ok(MigrationVerifyReport {
        expected_count,
        actual_count,
        matched_count,
        missing_count,
        missing_ids,
        count_match,
        with_cover,
        with_background: with_bg,
        with_description: with_desc,
        cover_rate,
        issues,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationVerifyReport {
    pub expected_count: usize,
    pub actual_count: usize,
    pub matched_count: usize,
    pub missing_count: usize,
    pub missing_ids: Vec<String>,
    pub count_match: bool,
    pub with_cover: usize,
    pub with_background: usize,
    pub with_description: usize,
    pub cover_rate: f64,
    pub issues: Vec<String>,
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_chinese_markers_standard() {
        let (name, desc) = parse_chinese_markers(
            "Some notes <!--moe:cn name=\"夏日口袋\" desc=\"美好的夏天\">--> more notes",
        );
        assert_eq!(name.as_deref(), Some("夏日口袋"));
        assert_eq!(desc.as_deref(), Some("美好的夏天"));
    }

    #[test]
    fn test_parse_chinese_markers_no_desc() {
        let (name, desc) = parse_chinese_markers("<!--moe:cn name=\"CLANNAD\"-->");
        assert_eq!(name.as_deref(), Some("CLANNAD"));
        assert_eq!(desc, None);
    }

    #[test]
    fn test_parse_chinese_markers_no_marker() {
        let (name, desc) = parse_chinese_markers("Just regular notes");
        assert_eq!(name, None);
        assert_eq!(desc, None);
    }

    #[test]
    fn test_parse_completion_status() {
        use crate::models::CompletionStatus;
        assert_eq!(
            parse_completion_status(Some("Playing")),
            CompletionStatus::Playing
        );
        assert_eq!(
            parse_completion_status(Some("Completed")),
            CompletionStatus::Completed
        );
        assert_eq!(parse_completion_status(None), CompletionStatus::NotStarted);
    }

    #[test]
    fn test_map_csharp_to_game_basic() {
        let cg = CSharpGame {
            id: "test-id-1".into(),
            name: "Test Game".into(),
            sort_name: None,
            description: Some("A test".into()),
            cover_image: None,
            background_image: None,
            icon: None,
            install_directory: Some("C:\\Games\\Test".into()),
            game_image_path: Some("C:\\Games\\Test\\game.exe".into()),
            source: Some("local".into()),
            platform: Some("Windows PC".into()),
            plugin_id: None,
            game_id: None,
            tags: None,
            genres: Some(vec!["Visual Novel".into()]),
            developers: Some(vec!["Key".into()]),
            publishers: None,
            release_date: Some("2020-06-01".into()),
            release_year: Some(2020),
            version: None,
            series: None,
            age_rating: Some("All Ages".into()),
            community_score: Some(8.5),
            playtime: Some(3600),
            last_activity: Some("2024-01-15 20:00".into()),
            completion_status: Some("Completed".into()),
            favorite: Some(true),
            hidden: None,
            notes: Some("Great game <!--moe:cn name=\"测试游戏\" desc=\"一个测试\">-->".into()),
            added: Some("2023-01-01 00:00".into()),
            modified: Some("2024-01-01 00:00".into()),
            original_name: Some("テストゲーム".into()),
            name_cn: None,
            description_cn: None,
            engine: Some("Unity".into()),
            homepage: None,
            vndb_id: Some("v123".into()),
            bangumi_id: None,
            vndb_rating: Some(8.0),
            bangumi_rating: None,
            estimated_hours: Some(30.0),
            install_dir_locale_jp: None,
        };

        let tmp = std::env::temp_dir().join("moegame_test_migration");
        let covers = tmp.join("covers");
        let bgs = tmp.join("backgrounds");
        fs::create_dir_all(&covers).ok();
        fs::create_dir_all(&bgs).ok();

        let icons = tmp.join("icons");
        fs::create_dir_all(&icons).ok();

        let (game, _, _) = map_csharp_to_game(&cg, None, &covers, &bgs, &icons).unwrap();
        assert_eq!(game.name, "Test Game");
        assert_eq!(game.metadata.developer.as_deref(), Some("Key"));
        assert_eq!(game.metadata.engine.as_deref(), Some("Unity"));
        assert_eq!(game.metadata.vndb_id.as_deref(), Some("v123"));
        assert_eq!(game.metadata.release_year, Some(2020));
        assert!(game.favorite);
        assert_eq!(
            game.play_tracker.completion_status,
            crate::models::CompletionStatus::Completed
        );
        // 中文标记应该被解析
        assert!(
            game.aliases.iter().any(|a| a.name == "测试游戏"),
            "should have chinese name alias"
        );

        // 清理
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn verify_migration_uses_source_ids_when_library_has_extra_games() {
        let db = SqliteDb::open_in_memory().unwrap();
        let mut migrated = Game::new("Migrated".into(), "migrated.exe".into());
        migrated.id = "legacy-1".into();
        migrated.cover = Some("covers/legacy-1.jpg".into());
        migrated.background = Some("backgrounds/legacy-1.jpg".into());
        let mut extra = Game::new("Extra".into(), "extra.exe".into());
        extra.id = "extra-1".into();
        db.import_games(&[migrated, extra]).unwrap();

        let verify = verify_migration_for_ids(&db, 1, &[String::from("legacy-1")]).unwrap();

        assert!(verify.count_match);
        assert_eq!(verify.actual_count, 2);
        assert_eq!(verify.matched_count, 1);
        assert_eq!(verify.missing_count, 0);
        assert_eq!(verify.with_cover, 1);
        assert!(verify.issues.is_empty());
    }

    #[test]
    fn backup_existing_database_snapshots_json_and_manifest() {
        let tmp =
            std::env::temp_dir().join(format!("moeplay_migration_backup_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("moegame.db"), b"db").unwrap();
        fs::write(tmp.join("database.json"), br#"{"keep":true}"#).unwrap();
        fs::write(tmp.join("manifest.json"), br#"{"version":1}"#).unwrap();

        let backup = backup_existing_database(&tmp)
            .unwrap()
            .expect("backup directory should be created");

        assert_eq!(fs::read(backup.join("moegame.db")).unwrap(), b"db");
        assert_eq!(
            fs::read_to_string(backup.join("database.json")).unwrap(),
            r#"{"keep":true}"#
        );
        assert_eq!(
            fs::read_to_string(backup.join("manifest.json")).unwrap(),
            r#"{"version":1}"#
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    #[ignore = "requires the local MoeGameSetup LiteDB library and imports hundreds of games"]
    fn smoke_import_local_moegame_litedb_library() {
        let Some(local_dir) = dirs::data_local_dir() else {
            eprintln!("no local data directory");
            return;
        };
        let library = local_dir.join("MoeGameSetup").join("library");
        let litedb = local_dir.join("MoeGameSetup").join("LiteDB.dll");
        if !library.join("games.db").is_file() || !litedb.is_file() {
            eprintln!("local MoeGameSetup LiteDB library is not present");
            return;
        }

        let stamp = chrono::Utc::now().timestamp_millis();
        let tmp = std::env::temp_dir().join(format!("moeplay_litedb_smoke_{stamp}"));
        fs::create_dir_all(&tmp).unwrap();
        let db = SqliteDb::open(tmp.join("moegame.db")).unwrap();
        let report = migrate_from_csharp(&library, &db, &tmp, &|_| {}).unwrap();
        let verify = verify_migration(&db, report.total_found).unwrap();

        assert!(
            report.total_found >= 500,
            "expected local legacy library, got {} games",
            report.total_found
        );
        assert_eq!(report.skipped, 0, "migration errors: {:?}", report.errors);
        assert_eq!(report.total_found, verify.actual_count);
        assert!(
            verify.with_cover >= 450,
            "cover import unexpectedly low: {}",
            verify.with_cover
        );
        assert!(
            verify.with_background >= 450,
            "background import unexpectedly low: {}",
            verify.with_background
        );

        let _ = fs::remove_dir_all(&tmp);
    }
}
